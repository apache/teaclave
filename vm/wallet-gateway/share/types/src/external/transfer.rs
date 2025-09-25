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
use std::convert::TryInto;

use crate::external::{division_u128_u128, multiplication_f64_u128};
use crate::external::{AssetType, ExternalAddress};
use crate::serde_util;
use crate::share::{AccountId, WalletID};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FeeInfo {
    // in min units, e.g. wei, satoshi
    // note: for ETH, it is about 10 * 10^9 wei (10 Gwei), which is in the scope of f64
    // for BTC, it is about 1.0~20.0 sat/vbyte
    pub fee_rate: f64,
    #[serde(with = "serde_util::u128_string")]
    pub units: u128,
    pub asset_type: AssetType,
}

impl FeeInfo {
    pub fn new(fee_rate: f64, units: u128, asset_type: AssetType) -> Self {
        Self {
            fee_rate,
            units,
            asset_type,
        }
    }

    pub fn fee(&self) -> Result<CkAmount> {
        let fee = multiplication_f64_u128(self.fee_rate, self.units)?;
        Ok(CkAmount::new(fee, self.asset_type))
    }

    pub fn fee_rate(&self) -> f64 {
        self.fee_rate
    }

    pub fn fee_rate_try_to_u128(&self) -> Result<u128> {
        // for ETH, the fee_rate is integer, so we can try to convert it to u128
        self.fee_rate
            .to_string()
            .parse::<u128>()
            .map_err(|_| anyhow::anyhow!("fee_rate is not integer"))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CkTransferInfo {
    pub from_wallet: WalletID,   // from external client
    pub from_account: AccountId, // fulfilled by webapi
    pub to: ExternalAddress,
    pub amount: CkAmount,
    pub fee_info: FeeInfo,
}

impl CkTransferInfo {
    pub fn from_account(&self) -> AccountId {
        self.from_account.clone()
    }

    pub fn from_wallet(&self) -> WalletID {
        self.from_wallet.clone()
    }

    pub fn recipient_string(&self) -> String {
        self.to.to_string()
    }

    pub fn amount(&self) -> &CkAmount {
        &self.amount
    }

    pub fn asset_type(&self) -> AssetType {
        self.amount.asset_type()
    }

    // sending amount + fee, maybe in different asset type, e.g. sending USDT, fee in ETH
    pub fn total_spend(&self) -> Result<HashMap<AssetType, CkAmount>> {
        let mut result = HashMap::new();
        let fee = self.fee_info.fee()?;
        // if amount.asset_type == fee.asset_type, we can use try_add directly
        if self.amount.asset_type() == fee.asset_type() {
            let mut amount = self.amount;
            amount.try_add(&fee)?;
            result.insert(self.amount.asset_type(), amount);
        } else {
            // if amount.asset_type != fee.asset_type, we need to insert both amount and fee
            result.insert(self.amount.asset_type(), self.amount);
            result.insert(fee.asset_type(), fee);
        }

        Ok(result)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CkReversedTransferInfo {
    pub from: ExternalAddress,
    pub to_account: AccountId, // fulfilled by task
    pub to_wallet: WalletID,   // fulfilled by task
    pub amount: CkAmount,
}

// CkAmount is for internal use only
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct CkAmount {
    #[serde(with = "serde_util::u128_string")]
    value: u128,
    asset_type: AssetType,
}

impl CkAmount {
    pub fn zero(asset_type: AssetType) -> Self {
        Self {
            value: 0,
            asset_type,
        }
    }

    pub fn new(value: u128, asset_type: AssetType) -> Self {
        Self { value, asset_type }
    }

    pub fn value(&self) -> u128 {
        self.value
    }

    pub fn asset_type(&self) -> AssetType {
        self.asset_type
    }

    pub fn try_compare(&self, other: &CkAmount) -> Result<std::cmp::Ordering> {
        if self.asset_type != other.asset_type {
            anyhow::bail!(
                "asset type not match, current: {:?}, requested: {:?}",
                self.asset_type,
                other.asset_type
            )
        }
        Ok(self.value.cmp(&other.value))
    }

    pub fn try_add(&mut self, other: &CkAmount) -> Result<()> {
        if self.asset_type != other.asset_type {
            anyhow::bail!(
                "asset type not match, current: {:?}, requested: {:?}",
                self.asset_type,
                other.asset_type
            )
        }
        self.value = self
            .value
            .checked_add(other.value())
            .ok_or_else(|| anyhow::anyhow!("add overflow: {} + {}", self.value, other.value()))?;
        Ok(())
    }

    pub fn try_add_u128(&mut self, other: u128) -> Result<()> {
        self.value = self
            .value
            .checked_add(other)
            .ok_or_else(|| anyhow::anyhow!("add overflow: {} + {}", self.value, other))?;
        Ok(())
    }

    pub fn try_sub(&mut self, other: &CkAmount) -> Result<()> {
        if self.asset_type != other.asset_type {
            anyhow::bail!(
                "asset type not match, current: {:?}, requested: {:?}",
                self.asset_type,
                other.asset_type
            )
        }
        self.value = self
            .value
            .checked_sub(other.value())
            .ok_or_else(|| anyhow::anyhow!("{} is less than {}", self.value, other.value()))?;
        Ok(())
    }

    pub fn try_sub_u128(&mut self, other: u128) -> Result<()> {
        self.value = self
            .value
            .checked_sub(other)
            .ok_or_else(|| anyhow::anyhow!("{} is less than {}", self.value, other))?;
        Ok(())
    }

    // converting to email content
    pub fn try_to_f64(&self) -> Result<f64> {
        division_u128_u128(self.value, 10u128.pow(self.asset_type.config().decimals()))
    }

    // converting to bitcoin::Amount(u64)
    pub fn try_to_u64(&self) -> Result<u64> {
        self.value
            .try_into()
            .map_err(|_| anyhow::anyhow!("value {} is too large to convert to u64", self.value))
    }
}
