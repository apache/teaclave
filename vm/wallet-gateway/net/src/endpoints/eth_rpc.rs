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

use crate::params::infura::{parse_infura_json_rpc_response, InfuraRpcError, InfuraRpcRequest};
use crate::params::{construct_gas_estimation_params, EthCallParams};
use anyhow::{anyhow, bail, ensure, Result};
use ethabi::Contract;
use types::external::{AssetType, CkAmount, NetworkErrMsg, NetworkTxHash, TxSubmissionResult};
use types::share::CkSignature;
use types::share::{EthAddress, NetworkType};

pub struct EthRpcEndpoint {
    url: url::Url, // ethereum rpc url, e.g. infura
    client: reqwest::Client,
    contract_abi: Option<Contract>,
    network: NetworkType,
}

impl EthRpcEndpoint {
    pub fn new(url: url::Url, contract_abi: Option<Contract>, network: NetworkType) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
            contract_abi,
            network,
        }
    }

    async fn get_eth_balance(&self, address: &EthAddress) -> Result<u128> {
        let get_balance_req = serde_json::to_value(InfuraRpcRequest::<String>::new(
            "eth_getBalance".to_string(),
            vec![address.to_string(), "latest".to_string()],
        ))?;

        log::debug!("Requesting balance from network: {:?}", get_balance_req);
        let resp = self
            .client
            .post(self.url.clone())
            .json(&get_balance_req)
            .send()
            .await?;

        let balance_resp = match parse_infura_json_rpc_response(resp).await {
            Ok(resp) => resp,
            Err(err) => bail!(err),
        };

        let balance = u128::from_str_radix(
            balance_resp
                .result()
                .strip_prefix("0x")
                .ok_or_else(|| anyhow!("eth_getBalance strip error"))?,
            16,
        )?;
        Ok(balance)
    }

    pub async fn get_nonce(&self, address: &EthAddress) -> Result<u128> {
        let get_nonce_req = serde_json::to_value(InfuraRpcRequest::<String>::new(
            "eth_getTransactionCount".to_string(),
            vec![address.to_string(), "pending".to_string()],
        ))?;

        log::debug!("Requesting nonce from network: {:?}", get_nonce_req);
        let resp = self
            .client
            .post(self.url.clone())
            .json(&get_nonce_req)
            .send()
            .await?;

        let tx_count_resp = match parse_infura_json_rpc_response(resp).await {
            Ok(resp) => resp,
            Err(err) => bail!(err),
        };

        let tx_count = u128::from_str_radix(
            tx_count_resp
                .result()
                .strip_prefix("0x")
                .ok_or_else(|| anyhow!("eth_getTransactionCount strip error"))?,
            16,
        )?;
        Ok(tx_count)
    }

    pub async fn send_raw_transaction(&self, payload: CkSignature) -> Result<TxSubmissionResult> {
        let raw_tx = format!("0x{}", hex::encode(payload.as_bytes()));
        let send_raw_tx_req = serde_json::to_value(InfuraRpcRequest::<String>::new(
            "eth_sendRawTransaction".to_string(),
            vec![raw_tx],
        ))?;
        log::info!("send_raw_tx_req: {:?}", &send_raw_tx_req);
        let resp = self
            .client
            .post(self.url.clone())
            .json(&send_raw_tx_req)
            .send()
            .await?;

        match parse_infura_json_rpc_response(resp).await {
            Ok(resp) => {
                let tx_hash = NetworkTxHash::from(resp.result());
                Ok(TxSubmissionResult::Accepted(tx_hash))
            }
            Err(err) => {
                let inner_error = err.downcast::<InfuraRpcError>()?;
                match inner_error {
                    // It's implementation choice to decide whith error to shown to user
                    // others are treated as internal error, which will effect the Delegations
                    InfuraRpcError::RejectedByNetwork(msg) => {
                        Ok(TxSubmissionResult::Rejected(NetworkErrMsg::from(msg)))
                    }
                    _ => bail!(inner_error),
                }
            }
        }
    }

    async fn get_erc20_balance(&self, address: &EthAddress, asset_type: AssetType) -> Result<u128> {
        ensure!(
            self.contract_abi.is_some(),
            "contract_abi should not be none"
        );
        // we use unwrap here because we have checked the contract_abi is not None
        let contract = &self.contract_abi.as_ref().unwrap();
        let balance_of_function = contract.function("balanceOf")?;

        let balance_call_encoded = balance_of_function.encode_input(&[ethabi::Token::Address(
            ethabi::Address::from_slice(address.as_bytes()),
        )])?;

        let asset = asset_type.config();
        let token_address = asset
            .contract_address(&self.network)
            .ok_or(anyhow!("contract_address is none"))?;
        let eth_call_params = serde_json::to_value(EthCallParams {
            to: token_address,
            data: balance_call_encoded,
        })?;

        let get_balance_req = serde_json::to_value(InfuraRpcRequest::<serde_json::Value>::new(
            "eth_call".to_string(),
            vec![
                eth_call_params,
                serde_json::Value::String("latest".to_string()),
            ],
        ))?;
        log::debug!("get_balance_req: {:?}", &get_balance_req);

        // Send the request
        let resp = self
            .client
            .post(self.url.clone())
            .json(&get_balance_req)
            .send()
            .await?;

        let balance_resp = match parse_infura_json_rpc_response(resp).await {
            Ok(resp) => resp,
            Err(err) => bail!(err),
        };

        // The balance will be a hex string; parse it into a u128
        let raw_balance = balance_resp
            .result()
            .strip_prefix("0x")
            .ok_or(anyhow!("result strip error, without the 0x prefix"))?
            .to_owned();
        let balance = u128::from_str_radix(&raw_balance, 16)?;
        log::debug!("get_erc20_balance: {:?}", balance);

        Ok(balance)
    }

    pub async fn get_address_balance_for_asset(
        &self,
        address: &EthAddress,
        asset_type: AssetType,
    ) -> Result<u128> {
        if asset_type.is_evm_compatible() {
            if asset_type.is_erc20_compatible() {
                let balance = self.get_erc20_balance(address, asset_type).await?;
                log::info!(
                    "address: {} {:?} balance: {:?}",
                    address,
                    asset_type,
                    balance
                );
                Ok(balance)
            } else if asset_type.is_evm_native() {
                let balance = self.get_eth_balance(address).await?;
                log::info!(
                    "address: {} {:?} balance: {:?}",
                    address,
                    asset_type,
                    balance
                );
                Ok(balance)
            } else {
                bail!("Unsupported evm asset type: {:?}", asset_type);
            }
        } else {
            bail!("Unsupported asset type: {:?}", asset_type);
        }
    }

    pub async fn get_gas_price(&self) -> Result<u128> {
        let get_price_req = serde_json::to_value(InfuraRpcRequest::<serde_json::Value>::new(
            "eth_gasPrice".to_string(),
            vec![],
        ))?;
        log::debug!("Requesting gas price from network: {:?}", get_price_req);
        let resp = self
            .client
            .post(self.url.clone())
            .json(&get_price_req)
            .send()
            .await?;
        log::debug!("Response: {:?}", &resp);

        let gas_price_resp = match parse_infura_json_rpc_response(resp).await {
            Ok(resp) => resp,
            Err(err) => bail!(err),
        };
        let gas_price = u128::from_str_radix(
            gas_price_resp
                .result()
                .strip_prefix("0x")
                .ok_or_else(|| anyhow!("eth_gasPrice strip error"))?,
            16,
        )?;
        Ok(gas_price)
    }

    pub async fn estimate_gas(
        &self,
        from: [u8; 20],
        to: [u8; 20],
        amount: CkAmount,
    ) -> Result<u128> {
        ensure!(
            self.contract_abi.is_some(),
            "contract_abi should not be none"
        );
        let params = construct_gas_estimation_params(
            from,
            to,
            amount,
            self.contract_abi.as_ref().unwrap(),
            &self.network,
        )?;

        let gas_estimate_req = serde_json::to_value(InfuraRpcRequest::new(
            "eth_estimateGas".to_string(),
            vec![params],
        ))?;

        log::debug!(
            "Requesting gas estimate from network: {:?}",
            gas_estimate_req
        );
        let resp = self
            .client
            .post(self.url.clone())
            .json(&gas_estimate_req)
            .send()
            .await?;

        let gas_estimate_resp = match parse_infura_json_rpc_response(resp).await {
            Ok(resp) => resp,
            Err(err) => bail!(err),
        };
        let gas_estimate = u128::from_str_radix(
            gas_estimate_resp
                .result()
                .strip_prefix("0x")
                .ok_or_else(|| anyhow!("eth_estimateGas strip error"))?,
            16,
        )?;
        Ok(gas_estimate)
    }
}
