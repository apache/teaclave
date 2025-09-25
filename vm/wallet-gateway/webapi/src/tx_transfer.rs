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

use crate::coin_selection::{ChangePolicy, CoinSelectionStrategy, CoinSelector};
use anyhow::{anyhow, ensure, Result};
use bitcoin::{Amount, OutPoint, TxOut, Txid};
use client_info::{ClientFeeInfo, FeeEstimationRequest, FeeEstimationResponse};
use db_manager::StorageClient;
use std::str::FromStr;
use std::sync::Arc;
use storable::balance::BalanceInfo;
use storable::{
    account_to_wallet::AccountToWallet,
    synced_wallet::SyncedWalletInfo,
    utxo_record::{AddressUtxoRecords, UtxoRecord},
};
use types::external::{
    AssetType, CkAmount, CkTransferInfo, ContractAddress, Erc20TokenConfig, TransferAbiData,
};
use types::share::{
    AccountId, BtcAddress, BtcTransaction, ChangeInfo, CkNetwork, ClientBtcAddress, EthTransaction,
    MultiChainTransaction, NetworkType, RecipientInfo, UtxoInfo, WalletID,
};

pub struct TxTransferManager {
    network_type: NetworkType,
    erc20_config: Arc<Erc20TokenConfig>,
    db_client: Arc<StorageClient>,
}

impl TxTransferManager {
    pub fn new(
        network_type: NetworkType,
        erc20_config: Arc<Erc20TokenConfig>,
        db_client: Arc<StorageClient>,
    ) -> Self {
        Self {
            network_type,
            erc20_config,
            db_client,
        }
    }

    pub fn check_balance(
        &self,
        transfer_info: &CkTransferInfo,
        balance_info: &BalanceInfo,
    ) -> Result<()> {
        let total_spend = transfer_info.total_spend()?;
        // check each balance of asset type
        for (asset_type, spend) in total_spend.iter() {
            let balance = balance_info.get_raw_amount_for_asset(asset_type)?;
            ensure!(
                balance.try_compare(spend)? == std::cmp::Ordering::Greater,
                "Insufficient balance, requested: {:?}, available: {:?}",
                spend,
                balance
            );
        }
        Ok(())
    }

    pub async fn prepare_tx(
        &self,
        transfer_info: &CkTransferInfo,
        balance_info: &BalanceInfo,
    ) -> Result<MultiChainTransaction> {
        let asset_type = transfer_info.amount().asset_type();
        self.check_balance(transfer_info, balance_info)?;

        if asset_type.is_ethereum_chain() {
            if asset_type.is_erc20() {
                Ok(MultiChainTransaction::Eth(self.handle_erc20_transfer(
                    CkNetwork::Eth(self.network_type),
                    transfer_info,
                )?))
            } else if asset_type.is_eth_native() {
                Ok(MultiChainTransaction::Eth(self.handle_evm_transfer(
                    CkNetwork::Eth(self.network_type),
                    transfer_info,
                )?))
            } else {
                Err(anyhow!("Unsupported eth asset type: {:?}", asset_type))
            }
        } else if asset_type.is_bsc_chain() {
            if asset_type.is_bep20() {
                Ok(MultiChainTransaction::Eth(self.handle_erc20_transfer(
                    CkNetwork::Bsc(self.network_type),
                    transfer_info,
                )?))
            } else if asset_type.is_bsc_native() {
                Ok(MultiChainTransaction::Eth(self.handle_evm_transfer(
                    CkNetwork::Bsc(self.network_type),
                    transfer_info,
                )?))
            } else {
                Err(anyhow!("Unsupported bsc asset type: {:?}", asset_type))
            }
        } else if asset_type.is_bitcoin_chain() {
            Ok(MultiChainTransaction::Btc(
                self.handle_btc_transfer(transfer_info).await?,
            ))
        } else {
            Err(anyhow!("Unsupported asset type: {:?}", asset_type))
        }
    }

    // EVM-compatible native token transfer
    fn handle_evm_transfer(
        &self,
        network: CkNetwork,
        transfer_info: &CkTransferInfo,
    ) -> Result<EthTransaction> {
        let to_address = transfer_info.to.clone().try_take_eth()?;
        let legacy_tx = EthTransaction {
            chain: network.chain_id(),
            nonce: None, // will be set as current nonce when signing
            from_wallet: transfer_info.from_wallet(),
            from_account: transfer_info.from_account(),
            to: to_address,
            value: transfer_info.amount.value(),
            gas_price: transfer_info.fee_info.fee_rate_try_to_u128()?,
            gas: transfer_info.fee_info.units,
            data: vec![],
        };
        log::debug!("legacy_tx: {:?}", legacy_tx);
        Ok(legacy_tx)
    }

    fn handle_erc20_transfer(
        &self,
        network: CkNetwork,
        transfer_info: &CkTransferInfo,
    ) -> Result<EthTransaction> {
        let transfer_amount = transfer_info.amount();
        let asset_type = transfer_amount.asset_type();
        let contract_address: ContractAddress = asset_type
            .config()
            .contract_address(self.erc20_config.network_type())
            .ok_or(anyhow!(
                "Failed to get contract address for {:?}",
                asset_type
            ))?;

        let to = transfer_info.to.clone().try_take_eth()?;
        let data =
            TransferAbiData::new(to, transfer_amount.value(), self.erc20_config.erc20_abi())?
                .encoded();

        let legacy_tx = EthTransaction {
            chain: network.chain_id(),
            nonce: None,
            from_wallet: transfer_info.from_wallet(),
            from_account: transfer_info.from_account(),
            to: contract_address,
            value: 0,
            gas_price: transfer_info.fee_info.fee_rate_try_to_u128()?,
            gas: transfer_info.fee_info.units,
            data,
        };

        Ok(legacy_tx)
    }

    async fn handle_btc_transfer(&self, transfer_info: &CkTransferInfo) -> Result<BtcTransaction> {
        let fee_rate_in_sat = transfer_info.fee_info.fee_rate();

        let recipient_info = vec![RecipientInfo {
            address: ClientBtcAddress::try_from(
                transfer_info.to.clone().try_take_btc()?,
                self.network_type,
            )?,
            amount: Amount::from_sat(transfer_info.amount.try_to_u64()?),
        }];
        let (utxos, change_info, total_weights) = self
            .select_utxos(
                &transfer_info.from_wallet,
                recipient_info.clone(),
                fee_rate_in_sat,
            )
            .await?;
        log::info!("total_weights: {}", total_weights);

        let total_vbytes = total_weights as u128 / 4;
        ensure!(
            total_vbytes <= transfer_info.fee_info.units,
            "insufficient gas_units: {:?}, minimal required: {:?}",
            transfer_info.fee_info.units,
            total_vbytes
        );

        Ok(BtcTransaction {
            from_wallet: transfer_info.from_wallet(),
            from_account: transfer_info.from_account(),
            utxo_list: utxos,
            recipient_list: recipient_info,
            change: change_info,
        })
    }

    pub async fn estimate_btc_tx_fee(
        &self,
        transfer_info: FeeEstimationRequest,
        fee_rate_in_sat: f64, // fee_rate fetched from db
    ) -> Result<FeeEstimationResponse> {
        let amount: CkAmount = transfer_info.amount.try_into()?;
        let recipient_info = RecipientInfo {
            address: ClientBtcAddress::try_from(transfer_info.to.as_str(), self.network_type)
                .map_err(|e| {
                    log::error!("failed to convert address: {:?}", e);
                    anyhow!("recipient address is not btc address")
                })?,
            amount: Amount::from_sat(amount.try_to_u64()?),
        };

        let (_utxos, _change_info, total_weights) = self
            .select_utxos(&transfer_info.from, vec![recipient_info], fee_rate_in_sat)
            .await?;

        let fee_info =
            ClientFeeInfo::new(fee_rate_in_sat, total_weights as u128 / 4, AssetType::BTC);
        Ok(FeeEstimationResponse { fee_info })
    }

    async fn select_utxos(
        &self,
        wallet: &WalletID,
        recipients: Vec<RecipientInfo>,
        fee_rate_in_sat: f64,
    ) -> Result<(Vec<UtxoInfo>, ChangeInfo, u32)> {
        let utxos: Vec<UtxoInfo> = self.get_all_utxos_of_wallet(wallet).await?;
        ensure!(
            !utxos.is_empty(),
            "insufficient balance for wallet: {:?}",
            wallet
        );
        let change_address: BtcAddress = self.get_change_address(wallet).await?;
        // ensure sum(utxo.value) > amount
        let amount = recipients
            .iter()
            .map(|recipient| recipient.amount)
            .sum::<bitcoin::Amount>();
        let total_amount_can_spend: bitcoin::Amount = utxos.iter().map(|utxo| utxo.value()).sum();
        log::info!("total_amount_can_spend: {:?}", total_amount_can_spend);
        ensure!(
            total_amount_can_spend > amount,
            "insufficient balance: {:?}, request: {:?}",
            total_amount_can_spend,
            amount
        );

        let coin_selector = CoinSelector::new(
            CoinSelectionStrategy::default(),
            ChangePolicy::default(),
            utxos,
            recipients,
        )?;

        let (selected_utxos, change_info, total_weights) =
            coin_selector.select(fee_rate_in_sat, change_address)?;
        log::info!("selected utxos: {:?}", selected_utxos,);
        log::info!("change_info: {:?}", change_info);
        let fee = selected_utxos
            .iter()
            .map(|utxo| utxo.value())
            .sum::<Amount>()
            - amount
            - change_info.amount;
        log::info!("total_weights: {:?}, fee: {:?}", total_weights, fee);

        Ok((selected_utxos, change_info, total_weights))
    }

    async fn get_all_utxos_of_wallet(&self, wallet_id: &WalletID) -> Result<Vec<UtxoInfo>> {
        let wallet = self.get_wallet_info(wallet_id).await?;
        // assume we only have one btc account for each wallet
        let account_id = wallet
            .btc_account()
            .ok_or(anyhow!("no btc account found"))?;
        self.get_all_utxos_of_account(&account_id).await
    }

    async fn get_all_utxos_of_account(&self, account_id: &AccountId) -> Result<Vec<UtxoInfo>> {
        let account_info = self
            .db_client
            .get::<_, AccountToWallet>(account_id)
            .await?
            .ok_or_else(|| anyhow!("account not found"))?;
        let addresses = account_info.all_btc_addresses()?;
        log::info!("addresses: {:?}", addresses);
        let mut utxo_info_list: Vec<UtxoInfo> = vec![];
        for address in addresses {
            let utxos_of_address = self
                .db_client
                .get::<_, AddressUtxoRecords>(&(*address).to_string())
                .await?;
            let utxo_records = match utxos_of_address {
                Some(utxos) => utxos.utxos().clone(),
                None => {
                    log::info!("no utxos for address: {:?}", address);
                    continue;
                }
            };
            for record in utxo_records {
                let utxo = self
                    .convert_utxo_record_to_info(address.clone(), &record)
                    .await?;
                utxo_info_list.push(utxo);
            }
        }
        Ok(utxo_info_list)
    }

    async fn convert_utxo_record_to_info(
        &self,
        sender_address: BtcAddress,
        utxo_record: &UtxoRecord,
    ) -> Result<UtxoInfo> {
        let out_point: OutPoint = OutPoint {
            txid: Txid::from_str(&utxo_record.txid)?,
            vout: utxo_record.vout.try_into()?,
        };
        let script_pubkey = sender_address.script_pubkey();
        let pre_tx_out: TxOut = TxOut {
            value: Amount::from_sat(utxo_record.value),
            script_pubkey,
        };
        let path = sender_address.path().clone();

        Ok(UtxoInfo::new(
            out_point,
            pre_tx_out,
            ClientBtcAddress::from(sender_address),
            path,
        ))
    }

    async fn get_change_address(&self, wallet_id: &WalletID) -> Result<BtcAddress> {
        let wallet = self.get_wallet_info(wallet_id).await?;
        let account_id = wallet
            .btc_account()
            .ok_or(anyhow!("no btc account found"))?;
        let account_info = self
            .db_client
            .get::<_, AccountToWallet>(&account_id)
            .await?
            .ok_or_else(|| anyhow!("account not found"))?;
        account_info.account.btc_change_address()
    }

    async fn get_wallet_info(&self, wallet_id: &WalletID) -> Result<SyncedWalletInfo> {
        let wallet_cache = self
            .db_client
            .get::<_, SyncedWalletInfo>(wallet_id)
            .await?
            .ok_or(anyhow!("wallet not found, wallet id: {:?}", wallet_id))?;
        Ok(wallet_cache)
    }
}
