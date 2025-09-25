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

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use db_manager::StorageClient;
use net::{BtcRpcEndpoint, EthRpcEndpoint};
use notification::email_event::{NotifyEvent, NotifyEventInfo};
use proto::TaCommand;
use proto::{SignTransactionInput, SignTransactionOutput};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use storable::account_to_wallet::AccountToWallet;
use storable::delegation::TxDelegation;
use storable::legacy_tx::Transaction;
use storable::pending_tx::PendingTx;
use storable::tx_history::HistoryTx;
use task_exec::Executable;
use tls_client_processing::TlsClient;
use types::external::{AssetType, CkAccount, TxSubmissionResult};
use types::share::{AccountId, CkSignature, MultiChainTransaction, TransactionID};

pub struct TxDelegationTask {
    db_client: Arc<StorageClient>,
    tls_client: Arc<RwLock<TlsClient>>,
    eth_rpc: Arc<EthRpcEndpoint>,
    bsc_rpc: Arc<EthRpcEndpoint>,
    btc_rpc: Arc<BtcRpcEndpoint>,
}

impl TxDelegationTask {
    pub fn new(
        db_client: Arc<StorageClient>,
        tls_client: Arc<RwLock<TlsClient>>,
        eth_rpc: Arc<EthRpcEndpoint>,
        bsc_rpc: Arc<EthRpcEndpoint>,
        btc_rpc: Arc<BtcRpcEndpoint>,
    ) -> Self {
        Self {
            db_client,
            tls_client,
            eth_rpc,
            bsc_rpc,
            btc_rpc,
        }
    }

    async fn get_all_delegation_tasks(&self) -> Result<HashMap<TransactionID, TxDelegation>> {
        self.db_client
            .list_entries::<TransactionID, TxDelegation>()
            .await
    }

    async fn inner_exec(&self) -> Result<()> {
        for (_tx_id, tx_delegation) in self.get_all_delegation_tasks().await? {
            if tx_delegation.expired() {
                self.process_expired_delegation(tx_delegation).await?;
                continue;
            }
            self.process_delegation(tx_delegation).await?;
        }
        Ok(())
    }

    async fn process_expired_delegation(&self, tx_delegation: TxDelegation) -> Result<()> {
        let tx_id = tx_delegation.tx.get_id();

        let pending_tx = self
            .db_client
            .get::<_, PendingTx>(&tx_id)
            .await?
            .ok_or_else(|| {
                anyhow!(
                    "expired delegation: pending tx not found for tx_id: {:?}",
                    tx_id
                )
            })?;

        let pending_tx = if let PendingTx::DelegationStarted(tx) = pending_tx {
            PendingTx::DelegationExpired(tx)
        } else {
            bail!("expired delegation: pending tx is not DelegationStarted");
        };

        self.db_client.put(&pending_tx).await?;
        self.db_client
            .delete_entry::<TransactionID, TxDelegation>(&tx_id)
            .await
    }

    // delegation
    async fn process_delegation(&self, mut delegation: TxDelegation) -> Result<()> {
        if delegation.signed_payload.is_none() {
            log::info!("delegation: signed payload is none, sign tx");
            let signature = self.sign_tx_in_tee(&mut delegation.tx).await?;
            // update delegation in db
            delegation.signed_payload = Some(signature);
            self.db_client.put(&delegation).await?;
            log::info!("delegation: sign_tx success, put into cache");
        }

        let tx_id = delegation.tx.get_id();

        let history_tx = self.send_to_network(delegation).await?;
        log::info!("delegation: send_to_network success");

        // notify
        let event = NotifyEvent::HistoryEvent(history_tx);
        self.db_client.put(&NotifyEventInfo::new(event)).await?;

        // remove delegation as we have finalized it to network, Txdelegation is destructed
        self.db_client
            .delete_entry::<TransactionID, TxDelegation>(&tx_id)
            .await
    }

    async fn get_account(&self, account_id: &AccountId) -> Result<CkAccount> {
        let a2w = self.db_client.get::<_, AccountToWallet>(account_id).await?;
        match a2w {
            Some(account) => Ok(account.account),
            None => Err(anyhow!("account not found")),
        }
    }

    async fn sign_tx_in_tee(&self, tx: &mut Transaction) -> Result<CkSignature> {
        let account_id = tx.transfer_info().from_account();
        let account = self.get_account(&account_id).await?;
        let asset_type = tx.transfer_info().asset_type();
        let tx_id = tx.get_id();

        let mc_tx = tx.get_multichain_tx_mut();

        self.prepare_tx_for_signing(account, mc_tx, asset_type)
            .await?;
        log::info!(
            "TxDelegationTask: multichain tx prepared for signing: {:?}",
            mc_tx
        );
        let input: SignTransactionInput = SignTransactionInput {
            tx_id,
            tx: mc_tx.clone(),
        };
        let mut tls_client = self.tls_client.write().unwrap();
        let output: SignTransactionOutput = tls_client.invoke(input, TaCommand::SignTransaction)?;
        Ok(output.signed_tx)
    }

    async fn prepare_tx_for_signing(
        &self,
        from_account: CkAccount,
        tx: &mut MultiChainTransaction,
        asset_type: AssetType,
    ) -> Result<()> {
        match tx {
            MultiChainTransaction::Eth(eth_tx) => {
                let eth_account = from_account
                    .take_eth_account()
                    .ok_or_else(|| anyhow!("prepare_tx_for_signing(): eth account not found"))?;
                let address = eth_account.eth_address();

                // Use eth_rpc for ETH tx, and bsc_rpc for BSC tx
                if asset_type.is_ethereum_chain() {
                    let nonce = self.eth_rpc.get_nonce(&address).await?;
                    eth_tx.nonce = Some(nonce);
                } else if asset_type.is_bsc_chain() {
                    let nonce = self.bsc_rpc.get_nonce(&address).await?;
                    eth_tx.nonce = Some(nonce);
                } else {
                    bail!(
                        "prepare_tx_for_signing: unsupported asset type: {:?}",
                        asset_type
                    );
                }

                // get current gas price from network
                let current_gas_price = if asset_type.is_ethereum_chain() {
                    self.eth_rpc.get_gas_price().await?
                } else if asset_type.is_bsc_chain() {
                    self.bsc_rpc.get_gas_price().await?
                } else {
                    bail!(
                        "prepare_tx_for_signing: unsupported asset type: {:?}",
                        asset_type
                    );
                };

                if current_gas_price < eth_tx.gas_price {
                    log::info!(
                        "prepare_tx_for_signing: adjust gas price: limit: {}, current: {}",
                        eth_tx.gas_price,
                        current_gas_price
                    );
                    eth_tx.gas_price = current_gas_price;
                } else {
                    log::info!(
                        "prepare_tx_for_signing: gas price from network {} is larger than approved gas price {}, use approved gas price",
                        current_gas_price,
                        eth_tx.gas_price
                    );
                };

                Ok(())
            }
            MultiChainTransaction::Btc(_) => {
                // for BTC we don't need modify tx
                log::info!(
                    "prepare_tx_for_signing: BTC tx does not require nonce or gas price adjustment"
                );
                Ok(())
            }
        }
    }

    async fn move_from_pending_to_history(
        &self,
        tx: Transaction,
        sig: Option<CkSignature>,
        submitted_result: TxSubmissionResult,
    ) -> Result<HistoryTx> {
        let tx_id = tx.get_id();
        let history_tx = HistoryTx::from_submitted(tx, sig, submitted_result);

        self.db_client.put(&history_tx).await?;

        self.db_client
            .delete_entry::<TransactionID, PendingTx>(&tx_id)
            .await?;

        Ok(history_tx)
    }

    // send tx to network
    // handle infura response:
    // 1. success => archive tx as on_chain
    // 2. rejected => archive tx as rejected_by_network
    // 3. network connection error => cache tx into infura cache
    // 4. unknown error => cache tx into infura cache
    pub async fn send_to_network(&self, tx_delegation: TxDelegation) -> Result<HistoryTx> {
        let tx = tx_delegation.tx;
        let asset_type = tx.transfer_info().asset_type();
        let payload = tx_delegation
            .signed_payload
            .ok_or_else(|| anyhow!("send_to_network: signed payload is None"))?;

        let submission_result = if asset_type.is_ethereum_chain() {
            log::info!("send_to_network: asset type is ethereum chain");
            self.eth_rpc.send_raw_transaction(payload.clone()).await?
        } else if asset_type.is_bsc_chain() {
            log::info!("send_to_network: asset type is bsc chain");
            self.bsc_rpc.send_raw_transaction(payload.clone()).await?
        } else if asset_type.is_bitcoin_chain() {
            log::info!("send_to_network: asset type is bitcoin chain");
            self.btc_rpc.broadcast_transaction(payload.clone()).await?
        } else {
            bail!("unsupported asset type: {:?}", asset_type);
        };

        let history_tx = self
            .move_from_pending_to_history(tx, Some(payload), submission_result)
            .await?;

        Ok(history_tx)
    }
}

#[async_trait]
impl Executable for TxDelegationTask {
    async fn exec(&self) {
        if let Err(e) = self.inner_exec().await {
            log::error!("Failed to run TxDelegationTask: {:?}", e);
        }
    }
}
