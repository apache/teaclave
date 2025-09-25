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

use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use storable::utxo_record::{AddressUtxoRecords, UtxoRecord};
use types::{
    external::TxSubmissionResult,
    share::{CkNetwork, CkSignature, ClientBtcAddress, NetworkType},
};

use crate::params::UtxoQueryResult;

pub struct BtcRpcEndpoint {
    url: url::Url,
    client: reqwest::Client,
    network: NetworkType,
}

impl BtcRpcEndpoint {
    pub fn new(network: CkNetwork) -> Self {
        matches!(network, CkNetwork::Btc(_));
        Self {
            url: network.rpc_api_url(),
            client: reqwest::Client::new(),
            network: network.network_type(),
        }
    }

    pub async fn get_utxos_for_address(
        &self,
        address: &ClientBtcAddress,
    ) -> Result<AddressUtxoRecords> {
        let esplora_get_utxo_endpoint =
            format!("{}/address/{}/utxo", self.url, address.address_str());
        log::debug!("esplora_get_utxo_endpoint: {:?}", esplora_get_utxo_endpoint);
        let response = self.client.get(esplora_get_utxo_endpoint).send().await;
        match response {
            Ok(res) => {
                let mut address_utxo_records = AddressUtxoRecords::new(address.clone());
                if res.status().is_success() {
                    let utxos: serde_json::Value = res.json().await?;
                    log::debug!("UTXOs: {:?}", utxos);
                    if utxos.is_array() {
                        for utxo in utxos.as_array().ok_or(anyhow!("Error parsing UTXOs"))? {
                            let result: UtxoQueryResult = match serde_json::from_value(utxo.clone())
                            {
                                Ok(result) => result,
                                Err(err) => {
                                    log::error!("Error parsing UTXO: {:?}", err);
                                    continue;
                                }
                            };
                            log::info!("UTXO - {:?}", result);

                            let utxo = UtxoRecord::new(
                                address.clone(),
                                &result.txid,
                                result.vout,
                                result.value,
                                result.status.block_height,
                                &result.status.block_hash,
                                result.status.block_time,
                            );
                            address_utxo_records.push(utxo);
                        }
                    } else {
                        log::info!("No UTXOs found for the address {:?}", address);
                    }
                } else {
                    bail!("Error in response: {}", res.status());
                }
                Ok(address_utxo_records)
            }
            Err(err) => {
                bail!("Error querying address information: {:?}", err);
            }
        }
    }

    pub async fn get_fee_rate(&self) -> Result<HashMap<u16, f64>> {
        let esplora_get_fee_rate_endpoint = format!("{}/fee-estimates", self.url);
        let response = self.client.get(esplora_get_fee_rate_endpoint).send().await;
        match response {
            Ok(res) => {
                if res.status().is_success() {
                    let fee_rates: HashMap<String, f64> = res.json().await?;
                    log::debug!("Estimated fee rate: {:?}", fee_rates);
                    // Convert the keys to u16
                    let fee_rates: HashMap<u16, f64> = fee_rates
                        .iter()
                        .map(|(k, v)| (k.parse().unwrap_or_default(), *v))
                        .collect();
                    Ok(fee_rates)
                } else {
                    log::error!("Error in response: {}", res.status());
                    Ok(HashMap::new())
                }
            }
            Err(err) => {
                bail!("Error querying fee rate: {:?}", err);
            }
        }
    }

    pub async fn broadcast_transaction(
        &self,
        serialized_tx: CkSignature,
    ) -> Result<TxSubmissionResult> {
        let esplora_broadcast_tx_endpoint = format!("{}/tx", self.url);
        let tx_hex: String = serialized_tx.into();
        let response = self
            .client
            .post(esplora_broadcast_tx_endpoint)
            .body(tx_hex)
            .send()
            .await;
        match response {
            Ok(res) => {
                if res.status().is_success() {
                    let res = res.text().await?;
                    log::info!("Transaction broadcasted successfully. Res: {:?}", res);
                    Ok(TxSubmissionResult::Accepted(res.into()))
                } else {
                    let msg = res.text().await?;
                    log::error!("Error in response: {}", msg);
                    Ok(TxSubmissionResult::Rejected(msg.into()))
                }
            }
            Err(err) => {
                bail!("Error broadcasting transaction: {:?}", err);
            }
        }
    }
}
