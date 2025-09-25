// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use crate::config::SharedStateConfig;
use crate::{AuthData, TxTransferManager};

use authority::UserRegistry;

use ck_config::BlockchainNetworkConfig;
use client_info::{
    AdditionalInfoForClient, ApprovalStageInfo, ClientFeeInfo, ClientTxTransfer,
    FeeEstimationRequest, FeeEstimationResponse, TxHistoryReceivedPayment, WalletInfoForClient,
    WalletNameInfo,
};
use credential_manager::CredentialManager;
use db_manager::{DBCompatibleClient, LocalServiceClient, StorageClient};
use notification::email_event::{NotifyEvent, NotifyEventInfo};
use proto::{
    ApproveTransactionInput, ApproveTransactionOutput, CreateTransactionInput,
    CreateTransactionOutput, RecallTransactionInput, RecallTransactionOutput, TaCommand,
};

use storable::address_book::{AddressBookEntry, AddressName};
use storable::console::ConsoleWalletInfo;
use storable::{
    account_to_wallet::AccountToWallet,
    balance::BalanceInfo,
    currency::AssetPriceInfo,
    delegation::{DelegationInfo, TxDelegation},
    fee::FeeEstimationInfo,
    gas_price::GasPriceInfo,
    legacy_tx::Transaction,
    nickname::{NicknameInfo, NicknameKey, NicknameValue},
    pending_tx::PendingTx,
    received_payment::{PaymentInfo, PaymentRecord},
    subscription::SubscriptionInfo,
    synced_wallet::SyncedWalletInfo,
    tx_history::HistoryTx,
    user_info::{Approver, TxOperator, User, UserInfo},
};
use tls_client_processing::TlsClient;

use types::external::{
    ApprovalChain, AssetType, CkAccount, CkAmount, CkReversedTransferInfo, CkTransferInfo,
    ClientExternalAddress, Email, Erc20TokenConfig, ExternalAddress,
};
use types::share::{
    AccountId, ApprovalOperation, CkNetwork, NetworkType, TaApprovalChain, TransactionID,
    TransactionStatus, WalletID,
};

use anyhow::{anyhow, bail, ensure, Result};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SharedStateManager {
    config: Arc<SharedStateConfig>,
    db_client: Arc<StorageClient>,
    user_registry: Arc<RwLock<UserRegistry>>,
    credential_manager: Arc<CredentialManager>,
    tx_transfer_manager: TxTransferManager,
}

// Internal helper functions for db operations
impl SharedStateManager {
    async fn get_wallet_info(&self, wallet_id: &WalletID) -> Result<SyncedWalletInfo> {
        let wallet_cache = self
            .db_client
            .get::<_, SyncedWalletInfo>(wallet_id)
            .await?
            .ok_or(anyhow!("wallet not found, wallet id: {:?}", wallet_id))?;
        Ok(wallet_cache)
    }

    async fn received_payments_for_account(
        &self,
        account: &AccountId,
        network_type: NetworkType,
    ) -> Result<Vec<PaymentRecord>> {
        let mut all_payments = Vec::new();
        for ck_network in CkNetwork::all_supported_networks(network_type) {
            let storage_key = format!("{}-{}", account, ck_network);
            if let Some(payment_info) = self.db_client.get::<_, PaymentInfo>(&storage_key).await? {
                all_payments.extend(payment_info.get_records_owned());
            }
        }
        Ok(all_payments)
    }

    async fn get_account_balance(&self, account: &AccountId) -> Result<Vec<CkAmount>> {
        let amounts =
            if let Some(balance_info) = self.db_client.get::<_, BalanceInfo>(account).await? {
                balance_info.take_amounts()
            } else {
                Vec::new()
            };

        Ok(amounts)
    }

    async fn get_asset_price_info(&self) -> Result<HashMap<AssetType, AssetPriceInfo>> {
        self.db_client
            .list_entries::<AssetType, AssetPriceInfo>()
            .await
    }

    async fn get_subscription_info(&self, user: &User) -> Result<SubscriptionInfo> {
        let email = user.get_info().get_email();
        match self.db_client.get::<Email, SubscriptionInfo>(email).await? {
            Some(subscription_info) => Ok(subscription_info),
            None => Ok(SubscriptionInfo::new(email.clone())),
        }
    }

    async fn get_account(&self, account_id: &AccountId) -> Result<CkAccount> {
        let entry = self
            .db_client
            .get::<_, AccountToWallet>(account_id)
            .await?
            .ok_or_else(|| anyhow!("account not found in db, account: {:?}", account_id))?;
        Ok(entry.account)
    }

    async fn list_all_wallets(&self) -> Result<HashMap<WalletID, SyncedWalletInfo>> {
        self.db_client
            .list_entries::<WalletID, SyncedWalletInfo>()
            .await
    }

    async fn get_console_wallet_info(&self, wallet_id: &WalletID) -> Result<ConsoleWalletInfo> {
        self.db_client
            .get::<_, ConsoleWalletInfo>(wallet_id)
            .await?
            .ok_or(anyhow!("console wallet not found in db"))
    }

    // add delegation tx into db, and return latest wrapped pending tx
    async fn duplicate_pending_with_delegation_tx(&self, tx: Transaction) -> Result<()> {
        let delegation_info = DelegationInfo::new(
            300,                // default to 5 mins
            tx.get_gas_price(), // keep origin gas price and gas limit
            tx.get_gas_limit(),
        );

        let payload = TxDelegation::new(tx, delegation_info);
        log::info!("delegation_tx: {:?}", &payload);

        // save one copy to db
        self.db_client.put(&payload).await?;

        // overwrite tx state in db
        let pending_tx = PendingTx::DelegationStarted(payload.tx);
        self.db_client.put(&pending_tx).await?;

        Ok(())
    }
}

impl SharedStateManager {
    pub async fn new(config: Arc<SharedStateConfig>) -> Result<Self> {
        let db_client = Arc::new(StorageClient::new(Box::new(
            LocalServiceClient::init(config.db_server_url.as_str(), None).await?,
        )));

        // load user registry and credential manager
        let user_registry = UserRegistry::init(db_client.clone()).await?;
        log::debug!("user registry initialized");
        let credential_manager = CredentialManager::init(
            db_client.clone(),
            config.certs_path.to_str().unwrap_or_default(),
        )
        .await?;
        log::debug!("credential manager initialized");

        let erc20_config = Arc::new(Erc20TokenConfig::new(config.network_config.network_type)?);

        log::info!("Network type: {:?}", config.network_config.network_type,);
        // tx transfer manager
        let tx_transfer_manager = TxTransferManager::new(
            config.network_config.network_type,
            erc20_config.clone(),
            db_client.clone(),
        );

        Ok(Self {
            config,
            db_client,
            user_registry: Arc::new(RwLock::new(user_registry)),
            credential_manager: Arc::new(credential_manager),
            tx_transfer_manager,
        })
    }

    pub async fn authenticate<U: TryFrom<UserInfo>>(&self, auth_data: &AuthData) -> Result<U> {
        let email = auth_data
            .get_authorized_email(&self.config().auth_service_url)
            .await?;

        let registry = self.user_registry.read().await;
        registry.get(&email).await
    }

    pub async fn get_authenticated_info(&self, auth_data: &AuthData) -> Result<UserInfo> {
        let email = auth_data
            .get_authorized_email(&self.config().auth_service_url)
            .await?;
        let registry = self.user_registry.read().await;
        registry.get_info(&email).await
    }

    pub async fn create_tx(
        &self,
        operator: TxOperator,
        transfer_info: ClientTxTransfer,
        existing_entry: &AddressBookEntry,
    ) -> Result<TransactionID> {
        log::info!(
            "operator: {:?} is trying to create tx, wallet: {:?}",
            operator,
            transfer_info.from
        );

        let wallet_id = transfer_info.from;
        let wallet = self.get_wallet_info(&wallet_id).await?;
        ensure!(
            wallet.associated_with_user(operator.get_email()),
            "operator is not authorized to create tx for wallet: {:?}",
            wallet_id
        );

        // check if the wallet's available balance is enough
        let onhold_balance_map = self.get_onhold_balance_of_wallet(&wallet_id).await?;
        let asset_type = transfer_info.amount.asset_type();

        // check if frontend send the correct fee asset type
        let fee_asset_type = transfer_info.fee_info.asset_type;
        ensure!(
            asset_type.config().fee_asset_type() == fee_asset_type,
            "asset type of fee {:?} is not matched, should be {:?}",
            fee_asset_type,
            asset_type.config().fee_asset_type()
        );

        let account_id = if asset_type.is_evm_compatible() {
            wallet.eth_account()
        } else if asset_type.is_bitcoin_chain() {
            wallet.btc_account()
        } else {
            bail!("unsupported asset type: {:?}", asset_type);
        }
        .ok_or(anyhow!("{:?} account not found", asset_type))?;

        let onchain_balance = self
            .db_client
            .get::<_, BalanceInfo>(&account_id)
            .await?
            .ok_or(anyhow!("balance info not found"))?;
        let available_balance = onchain_balance.get_available_balances(onhold_balance_map)?;

        let ck_transfer_info = CkTransferInfo {
            from_wallet: wallet_id,
            from_account: account_id,
            to: transfer_info.to.to_external_address(&asset_type)?,
            amount: CkAmount::try_from(transfer_info.amount)?,
            fee_info: transfer_info.fee_info.try_into()?,
        };
        log::info!("ck_transfer_info: {:?}", &ck_transfer_info);

        let multichain_tx = self
            .tx_transfer_manager
            .prepare_tx(&ck_transfer_info, &available_balance)
            .await?;

        // check if address name up-to-date
        let address_name_from_db = self
            .db_client
            .get::<_, AddressBookEntry>(&transfer_info.to.to_string().to_lowercase())
            .await?
            .ok_or(anyhow!("recipient address doesn't have address name"))?;
        ensure!(
            address_name_from_db == *existing_entry,
            "recipient address name is outdated, please refresh the page"
        );

        // TlsClient is not shared in SharedStateManager because we should use the credential of the user
        // who operates on the API
        let creds = self
            .credential_manager
            .get_user_tls_credential(operator.get_email())
            .await?;
        let mut tls_client = TlsClient::new(creds, &self.config().tee_server_url)?;

        let input = CreateTransactionInput {
            tx: multichain_tx.clone(),
        };

        log::debug!("create threshold wallet transaction, input: {:?}", &input);
        let output: CreateTransactionOutput =
            tls_client.invoke(&input, TaCommand::CreateTransaction)?;

        log::info!(
            "operator {:?} create tx: {:?} successfully",
            operator,
            &multichain_tx
        );
        let tx = Transaction::create(
            output.tx_id.clone(),
            operator.get_email().clone(),
            ck_transfer_info,
            multichain_tx,
            ApprovalChain::from(wallet.approval_chain),
            address_name_from_db,
        )?;

        let pending_tx = PendingTx::PendingForApproval(tx);
        self.db_client.put(&pending_tx).await?;
        // notify
        let event = NotifyEvent::PendingEvent(pending_tx);
        self.db_client.put(&NotifyEventInfo::new(event)).await?;

        Ok(output.tx_id)
    }

    async fn validate_before_operating_pending_tx(
        &self,
        approver: &Approver,
        tx_id: &TransactionID,
        ac_snapshot: Vec<ApprovalStageInfo>,
    ) -> Result<Transaction> {
        let pending_tx = self
            .get_pending_tx_for_user(approver.get_email(), tx_id)
            .await?;
        let tx = match pending_tx {
            PendingTx::PendingForApproval(tx) => tx,
            _ => bail!("tx is not pending for approval"),
        };

        let approval_chain = tx.get_approval_chain();

        // check approver
        ensure!(
            approval_chain.ready_for_current_stage(approver.get_email()),
            "approver is not ready for current stage"
        );

        // check snapshot
        for (snapshot, stage) in ac_snapshot.iter().zip(approval_chain.iter()) {
            ensure!(
                snapshot.match_stage(stage),
                "approval chain snapshot is not matched with current stage"
            );
        }

        Ok(tx)
    }

    pub async fn approve_tx(
        &self,
        approver: &Approver,
        tx_id: &TransactionID,
        ac_snapshot: Vec<ApprovalStageInfo>,
    ) -> Result<TransactionID> {
        let mut tx = self
            .validate_before_operating_pending_tx(approver, tx_id, ac_snapshot)
            .await?;

        log::info!("approver: {:?} is trying to approve tx: {:?}", approver, tx);

        self.invoke_ta_tx_operation(&mut tx, approver.get_email(), ApprovalOperation::Approve)
            .await?;

        // update tx in db
        match tx.overall_status() {
            TransactionStatus::Approved => self.duplicate_pending_with_delegation_tx(tx).await?,
            TransactionStatus::PendingForApproval => {
                // update tx state in db
                let pending_tx = PendingTx::PendingForApproval(tx);
                self.db_client.put(&pending_tx).await?;
                // notify
                let event = NotifyEvent::PendingEvent(pending_tx.clone());
                self.db_client.put(&NotifyEventInfo::new(event)).await?;
            }
            _ => bail!("invalid tx status"),
        }

        Ok(tx_id.to_owned())
    }

    // approve/reject operations in ta
    async fn invoke_ta_tx_operation(
        &self,
        tx: &mut Transaction,
        user: &Email,
        operation: ApprovalOperation,
    ) -> Result<()> {
        let creds = self
            .credential_manager
            .get_user_tls_credential(user)
            .await?;
        let mut tls_client = TlsClient::new(creds, &self.config().tee_server_url)?;

        let approval_status: TaApprovalChain = tx.get_approval_chain().clone().into();
        let input = ApproveTransactionInput {
            tx_id: tx.get_id(),
            current_approval_chain: approval_status,
            operation,
        };

        let ta_cmd = match operation {
            ApprovalOperation::Approve => TaCommand::ApproveTransaction,
            ApprovalOperation::Reject => TaCommand::ApproveTransaction,
        };

        let output: ApproveTransactionOutput = tls_client.invoke(input, ta_cmd)?;

        let lastest_approve_status = output.latest_approval_chain;

        log::debug!(
            "threshold wallet transaction, op: {:?}, tx_details: {:?}",
            operation,
            &lastest_approve_status
        );

        tx.update_approval_chain(lastest_approve_status)?;
        Ok(())
    }

    pub async fn reject_tx(
        &self,
        approver: &Approver,
        tx_id: &TransactionID,
        ac_snapshot: Vec<ApprovalStageInfo>,
    ) -> Result<TransactionID> {
        let mut tx = self
            .validate_before_operating_pending_tx(approver, tx_id, ac_snapshot)
            .await?;

        log::info!(
            "approver: {:?} is trying to reject tx_id: {:?}",
            approver,
            tx_id
        );

        self.invoke_ta_tx_operation(&mut tx, approver.get_email(), ApprovalOperation::Reject)
            .await?;

        log::info!(
            "approver {:?} reject tx: {:?} successfully",
            approver,
            tx_id
        );

        // move tx into history
        let tx_history = HistoryTx::from_approver_rejected(tx);
        self.db_client.put(&tx_history).await?;

        // remove from pending list
        self.db_client.delete_entry::<_, PendingTx>(tx_id).await?;
        // notify
        let event = NotifyEvent::HistoryEvent(tx_history);
        self.db_client.put(&NotifyEventInfo::new(event)).await?;

        // return for client response
        Ok(tx_id.to_owned())
    }

    pub async fn recall_tx(
        &self,
        operator: &TxOperator,
        tx_id: &TransactionID,
    ) -> Result<TransactionID> {
        log::info!(
            "operator: {:?} is trying to recall tx_id: {:?}",
            operator,
            tx_id
        );
        let tx = match self
            .db_client
            .get::<TransactionID, PendingTx>(tx_id)
            .await?
        {
            Some(pending_tx) => match pending_tx {
                PendingTx::DelegationStarted(_) => {
                    bail!("tx is already delegated, cannot recall until delegation attempts failed")
                }
                PendingTx::DelegationExpired(tx)
                | PendingTx::PendingForApproval(tx)
                | PendingTx::ReadyForSigning(tx) => tx,
            },
            None => bail!("tx not found"),
        };
        ensure!(
            tx.get_operator() == operator.get_email(),
            "operator is not authorized as tx {:?} operator",
            tx_id
        );

        let approval_status: TaApprovalChain = tx.get_approval_chain().clone().into();
        let cred = self
            .credential_manager
            .get_user_tls_credential(operator.get_email())
            .await?;
        let mut tls_client = TlsClient::new(cred, &self.config().tee_server_url)?;

        let input = RecallTransactionInput {
            tx_id: tx.get_id(),
            current_approval_chain: approval_status,
        };
        let _output: RecallTransactionOutput =
            tls_client.invoke(input, TaCommand::RecallTransaction)?;

        log::info!(
            "operator {:?} recall tx: {:?} successfully",
            operator,
            tx_id
        );

        // move tx into history
        let tx_history = HistoryTx::from_operator_recalled(tx);
        self.db_client.put(&tx_history).await?;
        // remove from pending tx
        self.db_client.delete_entry::<_, PendingTx>(tx_id).await?;
        // notify
        let event = NotifyEvent::HistoryEvent(tx_history);
        self.db_client.put(&NotifyEventInfo::new(event)).await?;

        Ok(tx_id.to_owned())
    }

    async fn get_btc_fee_rate(&self, asset_type: &AssetType) -> Result<f64> {
        if let GasPriceInfo::Btc(price_info) = self.get_gas_price_info(asset_type).await? {
            Ok(price_info.get_recommended_fee_rate_in_sat())
        } else {
            log::error!("gas price for asset type: {:?} is not found", asset_type);
            Ok(0.0)
        }
    }

    async fn _get_fee_estimation_info(&self, asset_type: &AssetType) -> Result<FeeEstimationInfo> {
        self.db_client
            .get::<AssetType, FeeEstimationInfo>(asset_type)
            .await?
            .ok_or_else(|| anyhow!("fee estimation info not found"))
    }

    async fn get_gas_price_info(&self, asset_type: &AssetType) -> Result<GasPriceInfo> {
        let network_type = self.config().network_config.network_type;
        let ck_network = if asset_type.is_ethereum_chain() {
            CkNetwork::Eth(network_type)
        } else if asset_type.is_bsc_chain() {
            CkNetwork::Bsc(network_type)
        } else if asset_type.is_bitcoin_chain() {
            CkNetwork::Btc(network_type)
        } else {
            bail!("unsupported asset type: {:?}", asset_type);
        };

        self.db_client
            .get::<CkNetwork, GasPriceInfo>(&ck_network)
            .await?
            .ok_or_else(|| anyhow!("gas price info not found"))
    }

    pub async fn estimate_tx_fee(
        &self,
        transfer_info: FeeEstimationRequest,
    ) -> Result<FeeEstimationResponse> {
        let asset_type = transfer_info.amount.asset_type();
        // ensure the decimal is valid
        transfer_info.amount.validate_decimal()?;

        if asset_type.is_ethereum_chain() {
            self.eth_estimate_tx_fee(transfer_info).await
        } else if asset_type.is_bsc_chain() {
            self.bsc_estimate_tx_fee(transfer_info).await
        } else if asset_type.is_bitcoin_chain() {
            self.btc_estimate_tx_fee(transfer_info).await
        } else {
            bail!("unsupported asset type: {:?}", asset_type);
        }
    }

    async fn eth_estimate_tx_fee(
        &self,
        transfer_info: FeeEstimationRequest,
    ) -> Result<FeeEstimationResponse> {
        let asset_type = transfer_info.amount.asset_type();
        let gas_units = if asset_type.is_eth_native() {
            21000 // gas units for ETH transfer
        } else if asset_type.is_erc20() {
            80000 // gas units for ERC20 transfer
        } else {
            bail!("unsupported asset type: {:?}", asset_type);
        };
        if let GasPriceInfo::Eth(price_info) = self.get_gas_price_info(&asset_type).await? {
            let unit_price = price_info.get_recommended_gas_price()?;
            Ok(FeeEstimationResponse {
                fee_info: ClientFeeInfo::new(unit_price, gas_units, AssetType::ETH),
            })
        } else {
            log::error!("gas price for ETH is not found, return the default value 0.0");
            Ok(FeeEstimationResponse {
                fee_info: ClientFeeInfo::new(0.0, gas_units, AssetType::ETH),
            })
        }
    }

    async fn bsc_estimate_tx_fee(
        &self,
        transfer_info: FeeEstimationRequest,
    ) -> Result<FeeEstimationResponse> {
        let asset_type = transfer_info.amount.asset_type();
        let gas_units = if asset_type.is_bsc_native() {
            21000 // gas units for BNB transfer
        } else if asset_type.is_bep20() {
            80000 // gas units for BEP20 transfer
        } else {
            bail!("unsupported asset type: {:?}", asset_type);
        };
        if let GasPriceInfo::Bsc(price_info) = self.get_gas_price_info(&asset_type).await? {
            let unit_price = price_info.get_recommended_gas_price()?;
            Ok(FeeEstimationResponse {
                fee_info: ClientFeeInfo::new(unit_price, gas_units, AssetType::BNB),
            })
        } else {
            log::error!("gas price for BSC is not found, return the default value 0.0");
            Ok(FeeEstimationResponse {
                fee_info: ClientFeeInfo::new(0.0, gas_units, AssetType::BNB),
            })
        }
    }

    async fn btc_estimate_tx_fee(
        &self,
        transfer_info: FeeEstimationRequest,
    ) -> Result<FeeEstimationResponse> {
        let asset_type = transfer_info.amount.asset_type();
        ensure!(asset_type.is_bitcoin_chain(), "asset type is not btc");

        let fee_rate_in_sat = self.get_btc_fee_rate(&asset_type).await?;
        self.tx_transfer_manager
            .estimate_btc_tx_fee(transfer_info, fee_rate_in_sat)
            .await
    }

    // nickname
    pub async fn get_nickname(
        &self,
        user: &User,
        key: NicknameKey,
    ) -> Result<Option<NicknameValue>> {
        let email = user.get_info().get_email();
        match self.db_client.get::<Email, NicknameInfo>(email).await? {
            Some(nickname_info) => Ok(nickname_info.get_nickname(&key)),
            None => Ok(None),
        }
    }

    pub async fn set_nickname(
        &self,
        user: &User,
        key: NicknameKey,
        nickname: NicknameValue,
    ) -> Result<()> {
        let email = user.get_info().get_email();
        let mut nickname_info = match self.db_client.get::<Email, NicknameInfo>(email).await? {
            Some(nickname_info) => nickname_info,
            None => {
                log::debug!("set_nickname(): nickname info not found, create new one");
                NicknameInfo::new(email.clone())
            }
        };
        nickname_info.set_nickname(key, nickname);
        self.db_client.put(&nickname_info).await
    }

    pub async fn remove_nickname(&self, user: &User, key: &NicknameKey) -> Result<()> {
        let email = user.get_info().get_email();
        let mut nickname_info = match self.db_client.get::<Email, NicknameInfo>(email).await? {
            Some(nickname_info) => nickname_info,
            None => return Ok(()),
        };
        nickname_info.remove_nickname(key);
        self.db_client.put(&nickname_info).await
    }

    pub async fn get_all_nickname_for_user(
        &self,
        user: &User,
    ) -> Result<HashMap<NicknameKey, NicknameValue>> {
        let email = user.get_info().get_email();
        match self.db_client.get::<Email, NicknameInfo>(email).await? {
            Some(nickname_info) => Ok(nickname_info.take_all()),
            None => Ok(HashMap::new()),
        }
    }

    // subscribe
    pub async fn subscribe(&self, user: &User, wallet_id: &WalletID) -> Result<()> {
        let email = user.get_info().get_email();

        // check user is associated with this account
        let wallet = self.get_wallet_info(wallet_id).await?;
        let console_wallet_info = self.get_console_wallet_info(wallet_id).await?;
        ensure!(
            wallet.associated_with_user(email) || console_wallet_info.can_view(email),
            "user is not associated with this account"
        );

        let mut subscription_info = self.get_subscription_info(user).await?;
        subscription_info.subscribe(wallet_id);
        self.db_client.put(&subscription_info).await
    }

    pub async fn subscribe_all_associated_wallets(&self, user: &User) -> Result<()> {
        let email = user.get_info().get_email();
        let wallets = self.db_client.list_entries::<_, SyncedWalletInfo>().await?;

        let mut subscription_info =
            match self.db_client.get::<Email, SubscriptionInfo>(email).await? {
                Some(subscription_info) => subscription_info,
                None => SubscriptionInfo::new(email.clone()),
            };

        for wallet in wallets.into_values() {
            let console_wallet_info = self.get_console_wallet_info(wallet.id()).await?;
            if wallet.associated_with_user(email) || console_wallet_info.can_view(email) {
                subscription_info.subscribe(wallet.id());
            }
        }

        self.db_client.put(&subscription_info).await?;
        Ok(())
    }

    pub async fn unsubscribe(&self, user: &User, wallet_id: &WalletID) -> Result<()> {
        let mut subscription_info = self.get_subscription_info(user).await?;
        subscription_info.unsubscribe(wallet_id);
        self.db_client.put(&subscription_info).await
    }

    pub async fn unsubscribe_all(&self, user: &User) -> Result<()> {
        let email = user.get_info().get_email();
        self.db_client
            .delete_entry::<_, SubscriptionInfo>(email)
            .await
    }

    pub async fn add_address_book_entry(
        &self,
        user: &User,
        address: &ClientExternalAddress,
        name: &AddressName,
    ) -> Result<AddressBookEntry> {
        let email = user.get_info().get_email().to_owned();

        // ensure entry not exists
        ensure!(
            self.db_client
                .get::<_, AddressBookEntry>(&address.to_string())
                .await?
                .is_none(),
            "entry already exists"
        );
        let entry = AddressBookEntry::new(address.to_owned(), name.to_owned(), email);
        self.db_client.put(&entry).await?;
        Ok(entry)
    }

    pub async fn update_address_book_entry(
        &self,
        user: &User,
        existing_entry: &AddressBookEntry,
        new_name: &AddressName,
    ) -> Result<()> {
        let email = user.get_info().get_email().to_owned();
        // check if the existing_entry matches the one in db
        let mut entry_in_db = self
            .db_client
            .get::<_, AddressBookEntry>(&existing_entry.address.to_string().to_lowercase())
            .await?
            .ok_or(anyhow!("entry not found"))?;
        ensure!(
            entry_in_db == *existing_entry,
            "entry is outdated, please refresh the page"
        );
        // update
        entry_in_db.update_name(new_name.to_owned(), email);
        self.db_client.put(&entry_in_db).await
    }

    pub async fn remove_address_book_entry(
        &self,
        _user: &User,
        existing_entry: &AddressBookEntry,
    ) -> Result<()> {
        // check if the existing_entry matches the one in db
        let entry_in_db = self
            .db_client
            .get::<_, AddressBookEntry>(&existing_entry.address.to_string().to_lowercase())
            .await?
            .ok_or(anyhow!("entry not found"))?;
        ensure!(
            entry_in_db == *existing_entry,
            "entry is outdated, please refresh"
        );
        // delete
        self.db_client
            .delete_entry::<_, AddressBookEntry>(&existing_entry.address.to_string().to_lowercase())
            .await
    }

    pub async fn list_address_book_entries(&self, _user: &User) -> Result<Vec<AddressBookEntry>> {
        let entries = self.db_client.list_entries::<_, AddressBookEntry>().await?;
        Ok(entries.into_values().collect())
    }

    pub fn config(&self) -> &SharedStateConfig {
        &self.config
    }

    pub fn network_config(&self) -> &BlockchainNetworkConfig {
        &self.config.network_config
    }

    pub async fn gather_user_wallets_info(&self, user: &User) -> Result<Vec<WalletInfoForClient>> {
        let wallets = self.list_all_wallets().await?;
        let email = user.get_info().get_email();
        let prices = self.get_asset_price_info().await?;
        let subscription_info = self.get_subscription_info(user).await?;

        let mut wallets_info = vec![];

        for wallet in wallets.into_values() {
            let console_wallet_info = self.get_console_wallet_info(wallet.id()).await?;
            // wallet associated with user or user can view this wallet
            if !wallet.associated_with_user(email) && !console_wallet_info.can_view(email) {
                continue;
            }

            let is_subscribed = subscription_info.is_subscribed(wallet.id());
            let mut accounts = vec![];
            let mut balances = vec![];
            for mca in wallet.accounts() {
                let account_id = mca.id();
                let account = self.get_account(account_id).await?;
                let mut balance = self.get_account_balance(account_id).await?;
                accounts.push(account);
                balances.append(&mut balance);
            }

            // return available_balance = balance - onhold_balance
            let onhold_balance_map = self.get_onhold_balance_of_wallet(wallet.id()).await?;
            let mut available_balances: HashMap<AssetType, CkAmount> = HashMap::new();
            for amount in balances.clone().iter_mut() {
                let asset = amount.asset_type();
                match onhold_balance_map.get(&asset) {
                    Some(onhold_balance) => {
                        amount.try_sub(onhold_balance)?;
                        available_balances.insert(asset, *amount);
                    }
                    None => {
                        available_balances.insert(asset, *amount);
                    }
                }
            }

            // get wallet name
            let wallet_name = console_wallet_info.wallet_name.clone();
            let viewers = console_wallet_info.viewers.clone();
            let info_for_client = WalletInfoForClient::try_from(
                wallet,
                wallet_name,
                viewers,
                accounts,
                balances,
                available_balances,
                &prices,
                is_subscribed,
                self.network_config().network_type,
            )?;
            wallets_info.push(info_for_client);
        }

        Ok(wallets_info)
    }

    // get additional info for "from" and "to"
    pub async fn get_additional_info(
        &self,
        ck_transfer_info: &CkTransferInfo,
    ) -> Result<(AdditionalInfoForClient, AdditionalInfoForClient)> {
        // "from" is wallet id, get the wallet name
        let from_wallet_info = self
            .db_client
            .get::<_, ConsoleWalletInfo>(&ck_transfer_info.from_wallet)
            .await?
            .ok_or(anyhow!("wallet not found in db"))?;
        let from_wallet_name = from_wallet_info.wallet_name.clone();
        // "to" id external address, get the address book entry
        // the storage key of all address should be lowercase for consistency
        let to_address = ck_transfer_info.to.to_string().to_lowercase();
        let to_address_entry = match self
            .db_client
            .get::<_, AddressBookEntry>(&to_address)
            .await?
        {
            Some(entry) => entry,
            // some legacy txs doesn't have address book entry
            None => AddressBookEntry::new_null(ck_transfer_info.to.clone().into()),
        };
        Ok((
            AdditionalInfoForClient::WalletId(WalletNameInfo {
                id: ck_transfer_info.from_wallet.clone(),
                name: from_wallet_name,
            }),
            AdditionalInfoForClient::RegisteredExternalAddress(to_address_entry),
        ))
    }

    pub async fn get_additional_info_for_received(
        &self,
        ck_rev_transfer_info: &CkReversedTransferInfo,
    ) -> Result<(AdditionalInfoForClient, AdditionalInfoForClient)> {
        // "to" is wallet id
        let to_wallet_info = self
            .db_client
            .get::<_, ConsoleWalletInfo>(&ck_rev_transfer_info.to_wallet)
            .await?
            .ok_or(anyhow!("wallet not found in db"))?;
        let to_wallet_name = to_wallet_info.wallet_name.clone();
        let to_info = AdditionalInfoForClient::WalletId(WalletNameInfo {
            id: ck_rev_transfer_info.to_wallet.clone(),
            name: to_wallet_name,
        });

        // "from": if BTC, is tx_hash; else, is external address
        let from_info = match &ck_rev_transfer_info.from {
            ExternalAddress::Btc(_) => AdditionalInfoForClient::TransactionId,
            ExternalAddress::Eth(_) => {
                match self
                    .db_client
                    .get::<_, AddressBookEntry>(
                        &ck_rev_transfer_info.from.to_string().to_lowercase(),
                    )
                    .await?
                {
                    Some(entry) => AdditionalInfoForClient::RegisteredExternalAddress(entry),
                    None => AdditionalInfoForClient::UnknownExternalAddress,
                }
            }
        };
        Ok((from_info, to_info))
    }

    // onhold balance: amount of pending txs
    pub async fn get_onhold_balance_of_wallet(
        &self,
        wallet_id: &WalletID,
    ) -> Result<HashMap<AssetType, CkAmount>> {
        let mut onhold_balance_sum: HashMap<AssetType, CkAmount> = HashMap::new();
        let all_pending_txs = self
            .db_client
            .list_entries::<TransactionID, PendingTx>()
            .await?;
        for (_, pending_tx) in all_pending_txs.iter() {
            if pending_tx.tx().transfer_info.from_wallet() == *wallet_id {
                let asset_type = pending_tx.asset_type();
                // total spend of this tx
                let total_spend = pending_tx.total_spend()?;
                // sum up
                let mut sum = *onhold_balance_sum
                    .get(&asset_type)
                    .unwrap_or(&CkAmount::zero(asset_type));
                sum.try_add(
                    total_spend
                        .get(&asset_type)
                        .unwrap_or(&CkAmount::zero(asset_type)),
                )?;
                onhold_balance_sum.insert(asset_type, sum);
            }
        }
        Ok(onhold_balance_sum)
    }

    pub async fn gather_user_history_tx(
        &self,
        user: &User,
    ) -> Result<Vec<TxHistoryReceivedPayment>> {
        let wallets = self.list_all_wallets().await?;
        let mut received_payment_list: Vec<TxHistoryReceivedPayment> = Vec::new();

        for wallet in wallets.into_values() {
            // ensure associate with user or user can view this wallet
            let console_wallet_info = self.get_console_wallet_info(wallet.id()).await?;
            if !wallet.associated_with_user(user.get_email())
                && !console_wallet_info.can_view(user.get_email())
            {
                continue;
            }
            for account in wallet.accounts {
                let account_id = account.id();
                let received_payment_for_account = self
                    .received_payments_for_account(account_id, self.network_config().network_type)
                    .await?;
                for payment in received_payment_for_account.into_iter() {
                    let (from_info, to_info) = self
                        .get_additional_info_for_received(&payment.transfer_info)
                        .await?;
                    received_payment_list.push(TxHistoryReceivedPayment::new(
                        payment,
                        self.config().network_config.network_type,
                        from_info,
                        to_info,
                    ));
                }
            }
        }
        Ok(received_payment_list)
    }

    pub async fn pending_tx_associated_with_email(
        &self,
        email: &Email,
        pending_tx: &PendingTx,
    ) -> Result<bool> {
        let console_info = self
            .get_console_wallet_info(&pending_tx.tx().transfer_info.from_wallet())
            .await?;
        Ok(pending_tx.tx().associated_with_user(email) || console_info.can_view(email))
    }

    pub async fn list_pending_tx_for_user(&self, user: &User) -> Result<Vec<PendingTx>> {
        let mut result = vec![];
        let all_txs = self
            .db_client
            .list_entries::<TransactionID, PendingTx>()
            .await?;
        for (_, pending_tx) in all_txs.iter() {
            if self
                .pending_tx_associated_with_email(user.get_email(), pending_tx)
                .await?
            {
                result.push(pending_tx.clone());
            }
        }
        Ok(result)
    }

    pub async fn get_pending_tx_for_user(
        &self,
        email: &Email,
        tx_id: &TransactionID,
    ) -> Result<PendingTx> {
        match self
            .db_client
            .get::<TransactionID, PendingTx>(tx_id)
            .await?
        {
            Some(pending_tx) => {
                ensure!(
                    self.pending_tx_associated_with_email(email, &pending_tx)
                        .await?,
                    "user is not associated with tx: {:?}",
                    tx_id
                );
                Ok(pending_tx)
            }
            None => bail!("tx not found"),
        }
    }

    pub async fn signed_tx_associated_with_email(
        &self,
        email: &Email,
        tx: &HistoryTx,
    ) -> Result<bool> {
        let console_info = self
            .get_console_wallet_info(&tx.tx().transfer_info.from_wallet())
            .await?;
        Ok(tx.tx().associated_with_user(email) || console_info.can_view(email))
    }

    pub async fn list_signed_tx_for_user(&self, user: &User) -> Result<Vec<HistoryTx>> {
        let mut result = vec![];
        let all_txs = self
            .db_client
            .list_entries::<TransactionID, HistoryTx>()
            .await?;
        for (_, history) in all_txs.iter() {
            if self
                .signed_tx_associated_with_email(user.get_email(), history)
                .await?
            {
                result.push(history.clone());
            }
        }
        Ok(result)
    }

    pub async fn get_signed_tx_for_user(
        &self,
        user: &User,
        tx_id: &TransactionID,
    ) -> Result<HistoryTx> {
        let tx = match self
            .db_client
            .get::<TransactionID, HistoryTx>(tx_id)
            .await?
        {
            Some(tx) => tx,
            None => bail!("tx not found"),
        };
        ensure!(
            self.signed_tx_associated_with_email(user.get_email(), &tx)
                .await?,
            "user is not associated with tx: {:?}",
            tx_id
        );
        Ok(tx.clone())
    }
}
