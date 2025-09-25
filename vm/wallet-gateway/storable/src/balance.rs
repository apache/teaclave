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
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use types::external::AssetType;
use types::external::CkAmount;
use types::share::AccountId;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BalanceInfo {
    account: AccountId,
    inner: HashMap<AssetType, CkAmount>,
}

impl BalanceInfo {
    // for eth account: inner.key: [ETH, USDT, USDC]
    // for btc account: inner.key: [BTC]
    pub fn new(account: AccountId, asset_types: Vec<AssetType>) -> Self {
        let inner = asset_types
            .into_iter()
            .map(|asset| (asset, CkAmount::zero(asset)))
            .collect();
        Self { account, inner }
    }

    pub fn add_balance(&mut self, asset_type: AssetType, balance: CkAmount) {
        self.inner.insert(asset_type, balance);
    }

    pub fn get_raw_amount_for_asset(&self, asset_type: &AssetType) -> Result<CkAmount> {
        self.inner
            .get(asset_type)
            .context("Asset not found")
            .cloned()
    }

    // onchain - onhold
    pub fn get_available_balances(
        &self,
        onhold_balance_map: HashMap<AssetType, CkAmount>,
    ) -> Result<BalanceInfo> {
        let mut available_balances = HashMap::new();
        for (asset_type, onchain_amount) in self.inner.iter() {
            let mut available_amount = onchain_amount.clone();
            let onhold_value = match onhold_balance_map.get(asset_type) {
                Some(onhold) => onhold.value(),
                None => 0u128,
            };
            available_amount.try_sub_u128(onhold_value)?;
            available_balances.insert(*asset_type, available_amount);
        }
        Ok(BalanceInfo {
            account: self.account.clone(),
            inner: available_balances,
        })
    }

    pub fn take_records(self) -> HashMap<AssetType, CkAmount> {
        self.inner
    }

    pub fn take_amounts(self) -> Vec<CkAmount> {
        self.inner.into_values().collect()
    }
}

impl Storable<AccountId> for BalanceInfo {
    fn unique_id(&self) -> AccountId {
        self.account.clone()
    }
}
