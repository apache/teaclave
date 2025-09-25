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
use types::external::AssetType;

// Query currency price for all chain, ETH, ERC20, BTC, ..
pub struct AssetPriceEndpoint {
    client: reqwest::Client,
    end_point: Box<dyn GetAssetPriceService>,
}

impl AssetPriceEndpoint {
    pub fn new(url: url::Url, token: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            end_point: Box::new(CoinGeckoEndpoint::new(url, token)),
        }
    }

    pub async fn get_asset_price(&self, asset_type: AssetType) -> Result<f64> {
        self.end_point
            .get_asset_price(self.client.clone(), asset_type)
            .await
    }
}

#[async_trait]
pub trait GetAssetPriceService: Send + Sync {
    fn new(url: url::Url, token: String) -> Self
    where
        Self: Sized;
    async fn get_asset_price(&self, client: reqwest::Client, asset_type: AssetType) -> Result<f64>;
}

pub struct CoinGeckoEndpoint {
    url: url::Url,
    token: String,
}

#[async_trait]
impl GetAssetPriceService for CoinGeckoEndpoint {
    fn new(url: url::Url, token: String) -> Self {
        Self { url, token }
    }
    async fn get_asset_price(&self, client: reqwest::Client, asset_type: AssetType) -> Result<f64> {
        let asset = asset_type.config();
        let currency_id = asset.currency_id();
        let resp = client
            .get(self.url.clone())
            .query(&[
                ("ids", currency_id.as_str()),
                ("vs_currencies", "usd"),
                ("x_cg_demo_api_key", &self.token),
            ])
            .send()
            .await?;
        let resp_json: serde_json::Value = resp.json().await?;
        match resp_json[&currency_id]["usd"].as_f64() {
            Some(price) => {
                log::info!("get usd_currency: {:?} for {:?}", price, asset_type);
                Ok(price)
            }
            None => {
                log::error!(
                    "Failed to get price for {:?}, response: {:?}",
                    asset_type,
                    resp_json
                );
                Err(anyhow!("Failed to get price for {:?}", asset_type))
            }
        }
    }
}
