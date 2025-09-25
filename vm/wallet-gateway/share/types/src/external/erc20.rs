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

use crate::share::NetworkType;
use anyhow::{Context, Result};
use ethabi::Contract;

#[derive(Debug)]
pub struct Erc20TokenConfig {
    erc20_abi: Contract,
    network_type: NetworkType,
}

impl Erc20TokenConfig {
    pub fn new(network_type: NetworkType) -> Result<Self> {
        let bytes = include_bytes!("../../../../abi/erc20_abi.json");
        let erc20_abi = Contract::load(bytes.as_slice()).context("Failed to load erc20 abi")?;

        Ok(Self {
            erc20_abi,
            network_type,
        })
    }
    pub fn erc20_abi(&self) -> &Contract {
        &self.erc20_abi
    }
    pub fn network_type(&self) -> &NetworkType {
        &self.network_type
    }
}

pub struct TransferAbiData {
    pub to: [u8; 20],
    pub value: u128,
    pub encoded: Vec<u8>,
}

impl TransferAbiData {
    pub fn new(to: [u8; 20], value: u128, erc20_abi: &Contract) -> Result<Self> {
        let transfer_function = erc20_abi.function("transfer")?;
        let inputs = vec![
            ethabi::Token::Address(ethabi::Address::from_slice(&to)),
            ethabi::Token::Uint(value.into()),
        ];
        let encoded = transfer_function
            .encode_input(&inputs)
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(Self { to, value, encoded })
    }
    pub fn encoded(&self) -> Vec<u8> {
        self.encoded.clone()
    }
}
