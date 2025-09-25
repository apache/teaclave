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

use anyhow::{bail, ensure, Result};
use async_trait::async_trait;
use db_manager::StorageClient;
use net::params::PaymentQueryResult;
use net::EthReceivedPaymentEndpoint;
use notification::email_event::{NotifyEvent, NotifyEventInfo};
use std::collections::HashMap;
use std::sync::Arc;
use storable::account_to_wallet::AccountToWallet;
use storable::received_payment::{PaymentInfo, PaymentRecord};
use storable::utxo_record::{AddressUtxoRecords, UtxoRecord};
use task_exec::Executable;
use types::external::{
    AssetType, BtcAccount, CkAccount, CkAmount, CkReversedTransferInfo, ExternalAddress,
};
use types::share::{AccountId, CkNetwork, ClientBtcAddress, EthAddress, NetworkType, WalletID};

pub struct SyncPaymentInfoTask {
    db_client: Arc<StorageClient>,
    eth_query_endpoint: EthReceivedPaymentEndpoint,
    bsc_query_endpoint: EthReceivedPaymentEndpoint,
    network_type: NetworkType,
}
impl SyncPaymentInfoTask {
    pub fn new(
        db_client: Arc<StorageClient>,
        eth_endpoint: EthReceivedPaymentEndpoint,
        bsc_endpoint: EthReceivedPaymentEndpoint,
        network_type: NetworkType,
    ) -> Self {
        Self {
            db_client,
            eth_query_endpoint: eth_endpoint,
            bsc_query_endpoint: bsc_endpoint,
            network_type,
        }
    }

    async fn update_evm_payment_info(
        &self,
        address: EthAddress,
        to_account: &AccountId,
        to_wallet: &WalletID,
        network: CkNetwork,
    ) -> Result<()> {
        let storage_key = format!("{}-{}", to_account, network);
        match self.db_client.get::<_, PaymentInfo>(&storage_key).await? {
            Some(mut payment_info) => {
                // if already have PaymentInfo in db, query the new received payment
                let payment_records = self
                    .evm_get_new_received_payment_records(
                        &address,
                        to_account,
                        to_wallet,
                        payment_info.last_notified_tx_block_number,
                        network,
                    )
                    .await?;
                // send email notification for new payments
                for payment in &payment_records {
                    let event = NotifyEvent::ReceivedPayment(payment.clone());
                    let notify_event_info = NotifyEventInfo::new(event);
                    self.db_client.put(&notify_event_info).await?;
                    log::info!("NotifyEventInfo: {:?}", notify_event_info);
                }
                payment_info.add_new_records(payment_records);
                log::debug!("PaymentInfo: {:?}", payment_info);
                self.db_client.put(&payment_info).await?;
            }
            None => {
                // when server init, there is no PaymentInfo in db, we query the last tx block number.
                // payments before this block number will not be notified. This can avoid notifying old payments.
                let payment_records = self
                    .evm_get_new_received_payment_records(
                        &address, to_account, to_wallet, 0, network,
                    )
                    .await?;
                log::info!("received payment for account: {}", to_account);

                let mut payment_info = PaymentInfo::new(to_account.clone(), network);
                payment_info.add_new_records(payment_records);
                self.db_client.put(&payment_info).await?;

                log::info!("created new PaymentInfo for account: {}", to_account);
                return Ok(());
            }
        };

        Ok(())
    }

    async fn evm_get_new_received_payment_records(
        &self,
        address: &EthAddress,
        to_account: &AccountId,
        to_wallet: &WalletID,
        last_notified_tx_block_number: u64,
        network: CkNetwork,
    ) -> Result<Vec<PaymentRecord>> {
        match network {
            CkNetwork::Eth(_) => {
                let payment_list = self
                    .eth_get_received_payment_result(address, last_notified_tx_block_number + 1)
                    .await?;

                if payment_list.is_empty() {
                    log::info!("no received payment for eth address: {}", address);
                    return Ok(Vec::new());
                }
                eth_parse_query_result(to_account, to_wallet, payment_list).await
            }
            CkNetwork::Bsc(_) => {
                let payment_list = self
                    .bsc_get_received_payment_result(address, last_notified_tx_block_number + 1)
                    .await?;
                if payment_list.is_empty() {
                    log::info!("no received payment for bsc address: {}", address);
                    return Ok(Vec::new());
                }

                bsc_parse_query_result(to_account, to_wallet, payment_list).await
            }
            _ => {
                log::error!(
                    "Invalid network type for evm_get_new_received_payment_records: {:?}",
                    network
                );
                bail!("Invalid network type for evm_get_new_received_payment_records");
            }
        }
    }

    async fn eth_get_received_payment_result(
        &self,
        address: &EthAddress,
        last_notified_tx_block_number: u64,
    ) -> Result<Vec<PaymentQueryResult>> {
        let mut eth_received_payment_list = Vec::new();
        // get normal ETH and ERC20 payments
        for eth_assets in AssetType::all_eth_assets() {
            let asset_payment_list = self
                .eth_query_endpoint
                .get_received_payment(address, eth_assets, last_notified_tx_block_number, false)
                .await?;
            eth_received_payment_list.extend(asset_payment_list);
            std::thread::sleep(std::time::Duration::from_secs_f32(0.5));
        }
        // get internal transactions
        let internal_payment_list = self
            .eth_query_endpoint
            .get_received_payment(address, AssetType::ETH, last_notified_tx_block_number, true)
            .await?;
        eth_received_payment_list.extend(internal_payment_list);

        // sort as block_number desc
        eth_received_payment_list.sort_by(|a, b| b.block_number.cmp(&a.block_number));

        Ok(eth_received_payment_list)
    }

    async fn bsc_get_received_payment_result(
        &self,
        address: &EthAddress,
        last_notified_tx_block_number: u64,
    ) -> Result<Vec<PaymentQueryResult>> {
        let mut bsc_received_payment_list = Vec::new();
        for bsc_assets in AssetType::all_bsc_assets() {
            let asset_payment_list = self
                .bsc_query_endpoint
                .get_received_payment(address, bsc_assets, last_notified_tx_block_number, false)
                .await?;
            bsc_received_payment_list.extend(asset_payment_list);
        }
        // sort as block_number desc
        bsc_received_payment_list.sort_by(|a, b| b.block_number.cmp(&a.block_number));

        Ok(bsc_received_payment_list)
    }

    async fn update_btc_payment_info(
        &self,
        to_account: &BtcAccount,
        to_wallet: &WalletID,
        network_type: NetworkType,
    ) -> Result<()> {
        // only invoice addresses receive external payments
        let all_addresses = to_account.all_client_invoice_addresses();
        let storage_key = format!("{}-{}", to_account.id(), CkNetwork::Btc(network_type));
        match self.db_client.get::<_, PaymentInfo>(&storage_key).await? {
            Some(mut payment_info) => {
                // if already have PaymentInfo in db, query the new received payment
                for address in all_addresses {
                    let payment_records = self
                        .btc_get_new_payment_records(
                            &address,
                            &to_account.id(),
                            to_wallet,
                            payment_info.last_notified_tx_block_number,
                        )
                        .await?;
                    // send email notification for new payments
                    for payment in &payment_records {
                        let event = NotifyEvent::ReceivedPayment(payment.clone());
                        let notify_event_info = NotifyEventInfo::new(event);
                        self.db_client.put(&notify_event_info).await?;
                        log::info!("NotifyEventInfo: {:?}", notify_event_info);
                    }
                    payment_info.add_new_records(payment_records);
                    log::debug!("PaymentInfo: {:?}", payment_info);
                    self.db_client.put(&payment_info).await?;
                }
            }
            None => {
                // when server init, there is no PaymentInfo in db, we query the last tx block number.
                // payments before this block number will not be notified. This can avoid notifying old payments.
                let mut payment_info =
                    PaymentInfo::new(to_account.id().clone(), CkNetwork::Btc(network_type));
                for address in all_addresses {
                    let payment_records = self
                        .btc_get_new_payment_records(&address, &to_account.id(), to_wallet, 0)
                        .await?;
                    payment_info.add_new_records(payment_records);
                }
                self.db_client.put(&payment_info).await?;
                log::info!("created new PaymentInfo for account: {:?}", to_account.id());
            }
        };

        Ok(())
    }

    async fn btc_get_new_payment_records(
        &self,
        address: &ClientBtcAddress,
        to_account: &AccountId,
        to_wallet: &WalletID,
        last_notified_tx_block_number: u64,
    ) -> Result<Vec<PaymentRecord>> {
        let utxo_records = self
            .btc_get_new_utxos_for_address(
                address,
                last_notified_tx_block_number + 1, //query start the next block
            )
            .await?;
        let payment_records = combine_utxos_to_payment_record(to_account, to_wallet, utxo_records)?;
        log::info!(
            "received payment records: {:?} for address: {:?}",
            payment_records,
            address
        );
        Ok(payment_records)
    }

    async fn btc_get_new_utxos_for_address(
        &self,
        address: &ClientBtcAddress,
        last_notified_tx_block_number: u64,
    ) -> Result<Vec<UtxoRecord>> {
        let address_utxo_list = self
            .db_client
            .get::<_, AddressUtxoRecords>(&address.address())
            .await?;
        match address_utxo_list {
            Some(records) => Ok(records
                .utxos()
                .iter()
                .filter(|utxo| {
                    let n: u64 = utxo.block_height.into();
                    n > last_notified_tx_block_number
                })
                .cloned()
                .collect()),
            None => {
                log::info!("no utxo record for address: {:?}", address);
                Ok(Vec::new())
            }
        }
    }

    async fn inner_exec(&self) -> Result<()> {
        let acc2wallet_list = self.db_client.list_entries::<_, AccountToWallet>().await?;

        for account in acc2wallet_list.values() {
            match account.account() {
                CkAccount::Eth(eth_account) => {
                    self.update_evm_payment_info(
                        eth_account.eth_address(),
                        &eth_account.id(),
                        account.wallet_id(),
                        CkNetwork::Eth(self.network_type),
                    )
                    .await?;
                    self.update_evm_payment_info(
                        eth_account.eth_address(),
                        &eth_account.id(),
                        account.wallet_id(),
                        CkNetwork::Bsc(self.network_type),
                    )
                    .await?;
                }
                CkAccount::Btc(btc_account) => {
                    self.update_btc_payment_info(
                        btc_account,
                        account.wallet_id(),
                        self.network_type,
                    )
                    .await?;
                }
            }
            std::thread::sleep(std::time::Duration::from_secs_f32(0.5));
        }

        Ok(())
    }
}

#[async_trait]
impl Executable for SyncPaymentInfoTask {
    async fn exec(&self) {
        if let Err(e) = self.inner_exec().await {
            log::error!("Failed to update payment info: {:?}", e);
        }
    }
}

async fn eth_parse_query_result(
    to_account: &AccountId,
    to_wallet: &WalletID,
    payment_result_list: Vec<PaymentQueryResult>,
) -> Result<Vec<PaymentRecord>> {
    let mut payment_records = Vec::new();
    for item in payment_result_list {
        log::debug!("get_received_payment_list_from_etherscan: item: {:?}", item);
        let asset_type = match item.token_symbol.clone() {
            Some(token_symbol) => {
                let asset_type: AssetType = token_symbol.try_into()?;
                ensure!(
                    asset_type.is_erc20(),
                    "Invalid asset type: {:?}, should be ERC20",
                    asset_type
                );
                log::info!("ERC20 payment");

                asset_type
            }
            None => {
                log::info!("ETH payment");
                AssetType::ETH
            }
        };

        let payment_info =
            parse_query_result_inner(to_account, to_wallet, &item, asset_type).await?;
        payment_records.push(payment_info);
    }
    // sort as block_number desc
    payment_records.sort_by(|a, b| b.block_number.cmp(&a.block_number));
    Ok(payment_records)
}

async fn parse_query_result_inner(
    to_account: &AccountId,
    to_wallet: &WalletID,
    payment_result: &PaymentQueryResult,
    asset_type: AssetType,
) -> Result<PaymentRecord> {
    let value = payment_result.value.parse()?;
    let amount = CkAmount::new(value, asset_type);
    let transfer_info = CkReversedTransferInfo {
        from: ExternalAddress::try_from_eth(payment_result.from.as_str())?,
        to_account: to_account.clone(),
        to_wallet: to_wallet.clone(),
        amount,
    };

    Ok(PaymentRecord::new(
        transfer_info,
        payment_result.time_stamp.parse()?,
        payment_result.hash.clone(),
        payment_result.block_number.parse()?,
    ))
}

async fn bsc_parse_query_result(
    to_account: &AccountId,
    to_wallet: &WalletID,
    payment_result_list: Vec<PaymentQueryResult>,
) -> Result<Vec<PaymentRecord>> {
    let mut payment_records = Vec::new();
    for item in payment_result_list {
        log::debug!("get_received_payment_list_from_bscscan: item: {:?}", item);
        let asset_type = match item.token_symbol.clone() {
            Some(token_symbol) => {
                let asset_type: AssetType =
                    AssetType::bep20_asset_from_rpc_token_symbol(&token_symbol)?;
                log::info!("BSC BEP20 payment");

                asset_type
            }
            None => {
                log::info!("BSC native payment");
                AssetType::BNB
            }
        };
        let payment_info =
            parse_query_result_inner(to_account, to_wallet, &item, asset_type).await?;
        payment_records.push(payment_info);
    }
    // sort as block_number desc
    payment_records.sort_by(|a, b| b.block_number.cmp(&a.block_number));
    Ok(payment_records)
}

fn combine_utxos_to_payment_record(
    to_account: &AccountId,
    to_wallet: &WalletID,
    utxo_list: Vec<UtxoRecord>,
) -> Result<Vec<PaymentRecord>> {
    // each tx has one Payment Record, the utxos of the same tx should be combined
    let mut map: HashMap<String, PaymentRecord> = HashMap::new(); // tx_id(hash), PaymentRecord
    for utxo in utxo_list {
        let tx_id = utxo.txid;
        match map.get_mut(&tx_id) {
            Some(payment_record) => {
                // merge utxo values to the same tx
                payment_record.try_add(utxo.value.into())?;
            }
            None => {
                let reversed_transfer_info = CkReversedTransferInfo {
                    from: ExternalAddress::Btc(tx_id.clone()), // use tx_id as the from address for btc
                    to_account: to_account.clone(),
                    to_wallet: to_wallet.clone(),
                    amount: CkAmount::new(utxo.value.into(), AssetType::BTC),
                };
                let payment_record = PaymentRecord::new(
                    reversed_transfer_info,
                    utxo.block_time,
                    tx_id.clone(),
                    utxo.block_height.into(),
                );
                map.insert(tx_id.to_string(), payment_record);
            }
        }
    }
    Ok(map.values().cloned().collect())
}
