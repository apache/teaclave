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
use serde::{Deserialize, Serialize};
use types::external::AssetType;
use types::share::{EthAddress, NetworkType};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentQueryResult {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub value: String,
    pub time_stamp: String,
    pub block_number: String,
    pub token_symbol: Option<String>,
    pub is_error: Option<String>, // Some("0") is success, Some("1") is error; for ERC20 token tx, it's None
}

pub fn construct_payment_params(
    url: &mut url::Url,
    token: &str,
    address: &EthAddress,
    asset_type: &AssetType,
    network_type: &NetworkType,
    query_start_block_number: u64,
    is_eth_internal_tx: bool,
) -> Result<()> {
    let address = address.as_hex();
    match asset_type.is_erc20_compatible() {
        // query normal ETH tx
        false => {
            let action = match is_eth_internal_tx {
                true => "txlistinternal",
                false => "txlist",
            };
            url.query_pairs_mut()
                .append_pair("module", "account")
                .append_pair("action", action)
                .append_pair("address", &address)
                .append_pair("startblock", &query_start_block_number.to_string())
                .append_pair("endblock", "99999999")
                .append_pair("page", "1")
                .append_pair("offset", "100")
                .append_pair("sort", "desc")
                .append_pair("apikey", token);
        }
        // query ERC20 token tx
        true => {
            let asset_config = asset_type.config();
            let contract_address = asset_config
                .contract_address(network_type)
                .ok_or(anyhow!("Contract address is none for: {:?}", asset_type))?;
            url.query_pairs_mut()
                .append_pair("module", "account")
                .append_pair("action", "tokentx")
                .append_pair("address", &address)
                .append_pair("startblock", &query_start_block_number.to_string())
                .append_pair("endblock", "999999999")
                .append_pair("page", "1")
                .append_pair("offset", "100")
                .append_pair("sort", "desc")
                .append_pair(
                    "contractaddress",
                    &format!("0x{}", hex::encode(contract_address)),
                )
                .append_pair("apikey", token);
            log::info!("construct_payment_params: url: {}", url.as_str());
        }
    }
    Ok(())
}
