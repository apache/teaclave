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

use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use storable::account_to_wallet::AccountToWallet;

use db_manager::StorageClient;
use net::BtcRpcEndpoint;
use task_exec::Executable;
use types::share::ClientBtcAddress;

// only for BTC: sync all utxos from electrum server
pub struct SyncUtxosTask {
    db_client: Arc<StorageClient>,
    electrum_rpc: Arc<BtcRpcEndpoint>,
}

impl SyncUtxosTask {
    pub fn new(db_client: Arc<StorageClient>, electrum_rpc: Arc<BtcRpcEndpoint>) -> Self {
        Self {
            db_client,
            electrum_rpc,
        }
    }

    async fn get_all_addresses(&self) -> Result<Vec<ClientBtcAddress>> {
        let acc2wallet_list = self.db_client.list_entries::<_, AccountToWallet>().await?;
        Ok(acc2wallet_list
            .iter()
            .flat_map(|(_, acc2wallet)| acc2wallet.all_client_btc_addresses())
            .collect())
    }

    async fn inner_exec(&self) -> Result<()> {
        let all_addresses = self.get_all_addresses().await?;
        for address in all_addresses {
            let addr_utxo_records = self.electrum_rpc.get_utxos_for_address(&address).await?;
            log::info!(
                "UTXOs for address {:?} fetched, {} utxos in total",
                address,
                addr_utxo_records.utxos().len()
            );
            self.db_client.put(&addr_utxo_records).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Executable for SyncUtxosTask {
    async fn exec(&self) {
        if let Err(e) = self.inner_exec().await {
            log::error!("SyncUtxosTask failed: {:?}", e);
        }
    }
}
