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

use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use storable::utxo_record::AddressUtxoRecords;

use anyhow::{anyhow, Result};
use db_manager::StorageClient;
use net::EthRpcEndpoint;
use storable::{account_to_wallet::AccountToWallet, balance::BalanceInfo};
use task_exec::Executable;
use types::external::CkAmount;
use types::external::{AssetType, CkAccount};
use types::share::{AccountId, ClientBtcAddress, EthAddress};

pub struct SyncBalanceInfoTask {
    db_client: Arc<StorageClient>,
    eth_rpc: Arc<EthRpcEndpoint>,
    bsc_rpc: Arc<EthRpcEndpoint>,
}

impl SyncBalanceInfoTask {
    pub fn new(
        db_client: Arc<StorageClient>,
        eth_rpc: Arc<EthRpcEndpoint>,
        bsc_rpc: Arc<EthRpcEndpoint>,
    ) -> Self {
        Self {
            db_client,
            eth_rpc,
            bsc_rpc,
        }
    }

    async fn sync_evm_assets(&self, account_id: AccountId, address: EthAddress) -> Result<()> {
        // Sync ETH assets
        self.sync_eth_assets(account_id.clone(), address.clone())
            .await?;
        // Sync BSC assets
        self.sync_bsc_assets(account_id, address).await?;
        Ok(())
    }

    async fn sync_eth_assets(&self, account_id: AccountId, address: EthAddress) -> Result<()> {
        let mut balance_info =
            if let Some(balance_info) = self.db_client.get::<_, BalanceInfo>(&account_id).await? {
                balance_info
            } else {
                BalanceInfo::new(account_id, AssetType::all_eth_assets())
            };

        for asset_type in AssetType::all_eth_assets() {
            let balance = match self
                .eth_rpc
                .get_address_balance_for_asset(&address, asset_type)
                .await
            {
                Ok(balance) => balance,
                Err(e) => {
                    log::error!(
                        "Failed to get balance for address: {}, asset_type: {:?}, error: {:?}",
                        address,
                        asset_type,
                        e
                    );
                    0
                }
            };
            let ck_amount = CkAmount::new(balance, asset_type);
            balance_info.add_balance(asset_type, ck_amount);
        }
        self.db_client.put(&balance_info).await.unwrap_or_else(|e| {
            log::error!("Failed to put BalanceInfo: {:?}", e);
        });
        Ok(())
    }

    async fn sync_bsc_assets(&self, account_id: AccountId, address: EthAddress) -> Result<()> {
        let mut balance_info =
            if let Some(balance_info) = self.db_client.get::<_, BalanceInfo>(&account_id).await? {
                balance_info
            } else {
                BalanceInfo::new(account_id, AssetType::all_bsc_assets())
            };

        for asset_type in AssetType::all_bsc_assets() {
            let balance = match self
                .bsc_rpc
                .get_address_balance_for_asset(&address, asset_type)
                .await
            {
                Ok(balance) => balance,
                Err(e) => {
                    log::error!(
                        "Failed to get balance for address: {}, asset_type: {:?}, error: {:?}",
                        address,
                        asset_type,
                        e
                    );
                    0
                }
            };
            let ck_amount = CkAmount::new(balance, asset_type);
            balance_info.add_balance(asset_type, ck_amount);
        }
        self.db_client.put(&balance_info).await.unwrap_or_else(|e| {
            log::error!("Failed to put BalanceInfo: {:?}", e);
        });
        Ok(())
    }

    async fn all_addresses_of_account(
        &self,
        account_id: &AccountId,
    ) -> Result<HashSet<ClientBtcAddress>> {
        let acc2wallet = self
            .db_client
            .get::<_, AccountToWallet>(account_id)
            .await?
            .ok_or(anyhow!(
                "AccountToWallet not found for account_id: {}",
                account_id
            ))?;
        log::info!("AccountToWallet: {:?}", acc2wallet);
        match acc2wallet.account() {
            CkAccount::Btc(account) => Ok(account.all_client_addresses()),
            _ => Err(anyhow!("Account type is not BTC")),
        }
    }

    async fn get_btc_balance(&self, address: &ClientBtcAddress) -> Result<u64> {
        match self
            .db_client
            .get::<_, AddressUtxoRecords>(&address.address())
            .await?
        {
            Some(address_utxo_list) => Ok(address_utxo_list.total_balance()),
            None => {
                log::warn!(
                    "No UTXO records found for address: {}, set balance as 0",
                    address.address()
                );
                Ok(0)
            }
        }
    }

    async fn sync_btc_assets(&self, account_id: AccountId) -> Result<()> {
        let mut balance_info =
            if let Some(balance_info) = self.db_client.get::<_, BalanceInfo>(&account_id).await? {
                balance_info
            } else {
                BalanceInfo::new(account_id.clone(), vec![AssetType::BTC])
            };

        let mut btc_balance = 0;
        let addresses = self.all_addresses_of_account(&account_id).await?;
        for address in addresses.iter() {
            let balance = self.get_btc_balance(address).await?;
            log::info!(
                "BTC balance for address: {:?}, balance: {}",
                address,
                balance
            );
            btc_balance += balance;
        }
        balance_info.add_balance(
            AssetType::BTC,
            CkAmount::new(btc_balance.into(), AssetType::BTC),
        );
        self.db_client.put(&balance_info).await.unwrap_or_else(|e| {
            log::error!("Failed to put BalanceInfo: {:?}", e);
        });
        Ok(())
    }

    pub async fn inner_exec(&self) -> Result<()> {
        let acc2wallet_list = self.db_client.list_entries::<_, AccountToWallet>().await?;

        for (account_id, account) in acc2wallet_list {
            match account.account() {
                CkAccount::Eth(account) => {
                    let address = account.eth_address();
                    self.sync_evm_assets(account_id, address).await?;
                }
                CkAccount::Btc(_) => {
                    self.sync_btc_assets(account_id).await?;
                }
            }
            std::thread::sleep(std::time::Duration::from_secs_f32(0.5));
        }

        Ok(())
    }
}

#[async_trait]
impl Executable for SyncBalanceInfoTask {
    async fn exec(&self) {
        if let Err(e) = self.inner_exec().await {
            log::error!("Failed to execute SyncBalanceInfoTask: {:?}", e);
        }
    }
}
