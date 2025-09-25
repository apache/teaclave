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
use storable::currency::AssetPriceInfo;

use db_manager::StorageClient;
use net::AssetPriceEndpoint;
use task_exec::Executable;
use types::external::AssetType;

pub struct SyncAssetPriceTask {
    db_client: Arc<StorageClient>,
    endpoint: AssetPriceEndpoint,
}

impl SyncAssetPriceTask {
    pub fn new(db_client: Arc<StorageClient>, endpoint: AssetPriceEndpoint) -> Self {
        Self {
            db_client,
            endpoint,
        }
    }

    async fn inner_exec(&self) -> Result<()> {
        for asset_type in AssetType::all_assets() {
            let price = self.endpoint.get_asset_price(asset_type).await?;
            let price_info = AssetPriceInfo::new(asset_type, price);
            self.db_client.put(&price_info).await?;
            log::debug!("{:?} currency to USD updated: {:?}", asset_type, price);
        }
        Ok(())
    }
}

#[async_trait]
impl Executable for SyncAssetPriceTask {
    async fn exec(&self) {
        if let Err(e) = self.inner_exec().await {
            log::error!("Failed to get asset price: {:?}", e);
        }
    }
}
