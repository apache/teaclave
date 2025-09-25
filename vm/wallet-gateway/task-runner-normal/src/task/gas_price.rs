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

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use storable::gas_price::{BtcFeeRateInfo, EthGasPriceInfo, GasPriceInfo};

use db_manager::StorageClient;
use net::{BtcRpcEndpoint, EthRpcEndpoint};
use task_exec::Executable;

use types::share::{CkNetwork, NetworkType};

pub struct SyncGasPriceTask {
    db_client: Arc<StorageClient>,
    eth_rpc: Arc<EthRpcEndpoint>, // for eth
    bsc_rpc: Arc<EthRpcEndpoint>, // for bsc
    btc_rpc: Arc<BtcRpcEndpoint>, // for btc
    network_type: NetworkType,
}

impl SyncGasPriceTask {
    pub fn new(
        db_client: Arc<StorageClient>,
        eth_rpc: Arc<EthRpcEndpoint>,
        bsc_rpc: Arc<EthRpcEndpoint>,
        btc_rpc: Arc<BtcRpcEndpoint>,
        network_type: NetworkType,
    ) -> Self {
        Self {
            db_client,
            eth_rpc,
            bsc_rpc,
            btc_rpc,
            network_type,
        }
    }

    async fn update_evm_price(&self, network: CkNetwork) -> Result<()> {
        // Get current gas price from appropriate RPC endpoint
        let current_gas_price = match network {
            CkNetwork::Eth(_) => self.eth_rpc.get_gas_price().await?,
            CkNetwork::Bsc(_) => self.bsc_rpc.get_gas_price().await?,
            _ => return Err(anyhow!("Unsupported EVM network: {:?}", network)),
        };

        if let Some(mut price_info) = self.db_client.get::<_, GasPriceInfo>(&network).await? {
            // Update and save price info
            price_info.update_evm(current_gas_price)?;
            self.db_client.put(&price_info).await?;
            log::info!("GasPriceInfo updated for {:?}: {:?}", network, price_info);
            Ok(())
        } else {
            let price_info = match network {
                CkNetwork::Eth(_) => GasPriceInfo::Eth(EthGasPriceInfo::new(self.network_type)),
                CkNetwork::Bsc(_) => GasPriceInfo::Bsc(EthGasPriceInfo::new(self.network_type)),
                _ => return Err(anyhow!("Unsupported EVM network: {:?}", network)),
            };
            // Save new price info
            self.db_client.put(&price_info).await?;
            log::info!(
                "New GasPriceInfo created for {:?}: {:?}",
                network,
                price_info
            );
            // Return a new EthGasPriceInfo instance
            Ok(())
        }
    }

    async fn update_btc_price(&self) -> Result<()> {
        // get GasPriceInfo from db
        let mut btc_price_info = if let Some(price_info) = self
            .db_client
            .get::<_, GasPriceInfo>(&CkNetwork::Btc(self.network_type))
            .await?
        {
            price_info
                .take_btc()
                .ok_or(anyhow!("Failed to get BtcFeeRateInfo"))?
        } else {
            BtcFeeRateInfo::new(self.network_type)
        };

        let current_map = self.btc_rpc.get_fee_rate().await?;
        // update GasPriceInfo
        btc_price_info.update(current_map);
        log::info!("GasPriceInfo updated: {:?}", btc_price_info);

        // save GasPriceInfo to db
        self.db_client
            .put(&GasPriceInfo::Btc(btc_price_info))
            .await?;
        Ok(())
    }

    async fn inner_exec(&self) -> Result<()> {
        // for each account type
        match self
            .update_evm_price(CkNetwork::Eth(self.network_type))
            .await
        {
            Ok(_) => log::info!("Eth gas price updated"),
            Err(e) => log::error!("Failed to update eth gas price: {:?}", e),
        }
        match self
            .update_evm_price(CkNetwork::Bsc(self.network_type))
            .await
        {
            Ok(_) => log::info!("Bsc gas price updated"),
            Err(e) => log::error!("Failed to update bsc gas price: {:?}", e),
        }
        match self.update_btc_price().await {
            Ok(_) => log::info!("Btc gas price updated"),
            Err(e) => log::error!("Failed to update btc gas price: {:?}", e),
        }
        Ok(())
    }
}

// todo: fix eth/btc network related gas price info
#[async_trait]
impl Executable for SyncGasPriceTask {
    async fn exec(&self) {
        if let Err(e) = self.inner_exec().await {
            log::error!("Failed to execute SyncGasPriceTask: {:?}", e);
        }
    }
}
