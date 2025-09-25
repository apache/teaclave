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

use serde::{Deserialize, Serialize};
use types::external::{AssetType, CkAmount};
use types::serde_util;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceForClient {
    asset_type: AssetType,
    decimal: u32,
    #[serde(with = "serde_util::u128_string")]
    balance: u128,
    #[serde(with = "serde_util::u128_string")]
    available_balance: u128,
    #[serde(with = "serde_util::f64_string")]
    currency_to_usd: f64,
}

impl BalanceForClient {
    pub fn new(balance: CkAmount, available_balance: CkAmount, currency_to_usd: f64) -> Self {
        Self {
            asset_type: balance.asset_type(),
            decimal: balance.asset_type().config().decimals(),
            balance: balance.value(),
            available_balance: available_balance.value(),
            currency_to_usd,
        }
    }
}
