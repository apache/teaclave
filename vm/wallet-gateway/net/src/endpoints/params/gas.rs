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

use crate::params::infura::RpcParams;
use anyhow::{anyhow, bail, Result};
use ethabi::Contract;
use serde::{Deserialize, Serialize};
use serde_hex::{SerHex, StrictPfx};
use types::external::CkAmount;
use types::external::TransferAbiData;
use types::serde_util;
use types::share::NetworkType;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EstimateGasParams {
    #[serde(with = "SerHex::<StrictPfx>")]
    pub from: [u8; 20],
    #[serde(with = "SerHex::<StrictPfx>")]
    pub to: [u8; 20],
    #[serde(with = "serde_util::u128_hex")]
    pub value: u128,
    #[serde(with = "serde_util::u128_hex")]
    pub gas: u128,
    #[serde(with = "serde_util::u128_hex")]
    pub gas_price: u128,
    #[serde(with = "serde_util::bytes_hex")]
    pub data: Vec<u8>,
}
impl RpcParams for EstimateGasParams {}

pub fn construct_gas_estimation_params(
    from: [u8; 20],
    to: [u8; 20],
    amount: CkAmount,
    erc20_abi: &Contract,
    network: &NetworkType,
) -> Result<EstimateGasParams> {
    let asset_type = amount.asset_type();
    let value = amount.value();

    if !asset_type.is_evm_compatible() {
        bail!("unsupported asset type: {:?}", asset_type);
    }
    if asset_type.is_erc20_compatible() {
        let contract_address = asset_type
            .config()
            .contract_address(network)
            .ok_or_else(|| anyhow!("contract_address is none for: {:?}", asset_type))?;
        let data = TransferAbiData::new(to, value, erc20_abi)?.encoded();
        Ok(EstimateGasParams {
            from,
            to: contract_address,
            value: 0,
            gas: 100000, // set a large number for estimation to avoid "Rejected by network: gas required exceeds allowance"
            gas_price: 0,
            data,
        })
    } else if asset_type.is_evm_native() {
        // For native ETH or BSC transfer, we set the value and data to zero
        Ok(EstimateGasParams {
            from,
            to,
            value,
            gas: 100000, // set a large number for estimation to avoid "Rejected by network: gas required exceeds allowance"
            gas_price: 0,
            data: vec![],
        })
    } else {
        bail!("unsupported asset type: {:?}", asset_type);
    }
}
