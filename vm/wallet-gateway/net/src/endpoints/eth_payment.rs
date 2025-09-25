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
use types::{
    external::AssetType,
    share::{CkNetwork, EthAddress, NetworkType},
};

use crate::endpoints::params::{construct_payment_params, PaymentQueryResult};

// Query historical payment from etherscan
// Etherscan is enough for our use case, if we need to use other
// service as backend, we can define a trait like in AssetPriceEndpoint
pub struct EthReceivedPaymentEndpoint {
    url: url::Url,
    token: String,
    network_type: NetworkType,
    client: reqwest::Client,
}

impl EthReceivedPaymentEndpoint {
    pub fn new(network: CkNetwork, token: String) -> Self {
        Self {
            url: network.explorer_api_url(),
            token,
            network_type: network.network_type(),
            client: reqwest::Client::new(),
        }
    }

    // Get all new received payment of specific asset type, for one ETH address
    pub async fn get_received_payment(
        &self,
        address: &EthAddress,
        asset_type: AssetType,
        query_start_block_number: u64,
        is_eth_internal_tx: bool,
    ) -> Result<Vec<PaymentQueryResult>> {
        ensure!(
            asset_type.is_evm_compatible(),
            "Invalid asset type: {:?}",
            asset_type
        );

        // sleep for a while to avoid hitting the rate limit
        std::thread::sleep(std::time::Duration::from_secs(1));

        let mut url = self.url.clone();
        construct_payment_params(
            &mut url,
            &self.token,
            address,
            &asset_type,
            &self.network_type,
            query_start_block_number,
            is_eth_internal_tx,
        )?;
        let resp = self
            .client
            .get(url)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;
        let result = match resp.get("result") {
            Some(resp) => {
                log::debug!("get_received_payment_list_from_etherscan: resp: {:?}", resp);
                serde_json::from_value::<Vec<PaymentQueryResult>>(resp.clone())?
            }
            None => {
                bail!("Failed to get result from etherscan, resp: {:?}", resp);
            }
        };
        // filter out the result that is not to the address and is success (is Some("0") or None)
        let result: Vec<PaymentQueryResult> = result
            .into_iter()
            .filter(|x| {
                x.to == address.as_hex()
                    && (x.is_error.is_none() || x.is_error.as_deref() == Some("0"))
            })
            .collect();

        // double check the txreceipt_status for each new tx
        // for normal tx, we can get it from the previous query, but for internal tx and ERC20 tx we can't
        // so check it here for all tx, return the confirmed tx
        self.check_tx_receipt_status(&result).await
    }

    // Check the tx receipt status
    async fn check_tx_receipt_status(
        &self,
        txs: &[PaymentQueryResult],
    ) -> Result<Vec<PaymentQueryResult>> {
        let mut confirmed_result = Vec::new();
        for tx in txs {
            // sleep for a while to avoid hitting the rate limit
            std::thread::sleep(std::time::Duration::from_secs(1));

            let tx_hash = tx.hash.clone();
            let url = format!(
                "{}?module=transaction&action=gettxreceiptstatus&txhash={}&apikey={}",
                self.url.as_str(),
                tx_hash,
                self.token
            );
            let resp = match self
                .client
                .get(url)
                .send()
                .await?
                .json::<serde_json::Value>()
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    log::error!(
                        "Failed to get tx receipt status from etherscan, error: {:?}",
                        e
                    );
                    continue;
                }
            };
            let status = match resp.get("result") {
                Some(resp) => {
                    log::debug!("get_received_payment_list_from_etherscan: resp: {:?}", resp);
                    serde_json::from_value::<serde_json::Value>(resp.clone())?
                }
                None => {
                    log::error!("Failed to get result from etherscan, resp: {:?}", resp);
                    continue;
                }
            };
            match status.get("status") {
                Some(status) => {
                    if status.as_str().unwrap_or_default() == "1" {
                        log::info!("tx: {:?} is confirmed", tx_hash);
                        confirmed_result.push(tx.clone());
                    }
                }
                None => {
                    log::error!("Failed to get status from etherscan, resp: {:?}", status);
                }
            }
        }
        Ok(confirmed_result)
    }
}
