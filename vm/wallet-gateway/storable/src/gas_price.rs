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

use crate::Storable;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use types::{
    external::{division_f64_f64, u128_to_f64, AssetType},
    share::{CkNetwork, NetworkType},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum GasPriceInfo {
    Eth(EthGasPriceInfo),
    Btc(BtcFeeRateInfo),
    Bsc(EthGasPriceInfo),
}

impl GasPriceInfo {
    pub fn new(network: CkNetwork) -> Self {
        match network {
            CkNetwork::Eth(net) => Self::Eth(EthGasPriceInfo::new(net)),
            CkNetwork::Btc(net) => Self::Btc(BtcFeeRateInfo::new(net)),
            CkNetwork::Bsc(net) => Self::Eth(EthGasPriceInfo::new(net)),
        }
    }

    pub fn network(&self) -> CkNetwork {
        match self {
            GasPriceInfo::Eth(info) => CkNetwork::Eth(info.network),
            GasPriceInfo::Btc(info) => CkNetwork::Btc(info.network),
            GasPriceInfo::Bsc(info) => CkNetwork::Bsc(info.network),
        }
    }

    pub fn take_eth(&self) -> Option<EthGasPriceInfo> {
        match self {
            GasPriceInfo::Eth(info) => Some(info.clone()),
            _ => None,
        }
    }

    pub fn take_bsc(&self) -> Option<EthGasPriceInfo> {
        match self {
            GasPriceInfo::Bsc(info) => Some(info.clone()),
            _ => None,
        }
    }

    pub fn take_btc(&self) -> Option<BtcFeeRateInfo> {
        match self {
            GasPriceInfo::Btc(info) => Some(info.clone()),
            _ => None,
        }
    }

    pub fn update_evm(&mut self, current_gas_price: u128) -> Result<()> {
        match self {
            GasPriceInfo::Eth(info) => {
                info.update(current_gas_price);
                Ok(())
            }
            GasPriceInfo::Bsc(info) => {
                info.update(current_gas_price);
                Ok(())
            }
            _ => Err(anyhow::anyhow!("Not an EVM network")),
        }
    }
}

impl Storable<CkNetwork> for GasPriceInfo {
    fn unique_id(&self) -> CkNetwork {
        self.network()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EthGasPriceInfo {
    network: NetworkType,
    gas_price_history: VecDeque<u128>, // last 7 days of data
    max_in_history: u128,              // cached maximum to avoid O(n) scans
}

impl EthGasPriceInfo {
    pub fn new(network: NetworkType) -> Self {
        Self {
            network,
            gas_price_history: VecDeque::new(),
            max_in_history: 0,
        }
    }

    pub fn update(&mut self, current_gas_price: u128) {
        // Keep 7 days of data: 7 days * 24 hours * 60 minutes / 30 minutes = 336 entries
        const MAX_ENTRIES: usize = 336;

        if self.gas_price_history.len() >= MAX_ENTRIES {
            let removed = self.gas_price_history.pop_front().unwrap();
            // If we removed the max value, we need to recalculate
            if removed == self.max_in_history {
                self.recalculate_max();
            }
        }

        self.gas_price_history.push_back(current_gas_price);

        // Update max if new price is higher
        if current_gas_price > self.max_in_history {
            self.max_in_history = current_gas_price;
        }
    }

    fn recalculate_max(&mut self) {
        self.max_in_history = self.gas_price_history.iter().max().copied().unwrap_or(0);
    }

    pub fn get_max_of_history(&self) -> u128 {
        self.max_in_history
    }

    pub fn get_current_gas_price(&self) -> u128 {
        // Get the most recent gas price from history
        self.gas_price_history.back().copied().unwrap_or(0)
    }

    pub fn get_recommended_gas_price(&self) -> Result<f64> {
        u128_to_f64(self.get_max_of_history())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BtcFeeRateInfo {
    network: NetworkType,
    default_waiting_blocks: u16,
    fee_rate_list: HashMap<u16, f64>, // fee rate (f64) for each waiting blocks (u16)
}

impl BtcFeeRateInfo {
    pub fn new(network: NetworkType) -> Self {
        Self {
            network,
            default_waiting_blocks: 6,
            fee_rate_list: HashMap::new(),
        }
    }

    pub fn update(&mut self, list: HashMap<u16, f64>) {
        self.fee_rate_list = list;
    }

    pub fn get_fee_rate(&self, waiting_blocks: u16) -> f64 {
        *self.fee_rate_list.get(&waiting_blocks).unwrap_or(&0.0)
    }

    pub fn get_recommended_fee_rate_in_btc(&self) -> Result<f64> {
        let fee_rate_sat_per_vbytes = self.get_fee_rate(self.default_waiting_blocks);
        // convert to BTC per vbytes
        division_f64_f64(
            fee_rate_sat_per_vbytes,
            10u32.pow(AssetType::BTC.config().decimals()) as f64,
        )
    }

    pub fn get_recommended_fee_rate_in_sat(&self) -> f64 {
        self.get_fee_rate(self.default_waiting_blocks)
    }
}
