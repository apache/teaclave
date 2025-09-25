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
use types::external::ClientExternalAddress;
use types::external::{
    u128_to_f64, AssetType, CkAmount, CkReversedTransferInfo, CkTransferInfo, FeeInfo,
};
use types::serde_util;
use types::share::WalletID;

// ClientAmount is reserved for email content which should directly return
// balance as f64
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClientAmount {
    #[serde(with = "serde_util::u128_string")]
    value: u128,
    asset_type: AssetType,
    decimal: u32,
}

impl ClientAmount {
    pub fn new(value: u128, asset_type: AssetType) -> Self {
        Self {
            value,
            asset_type,
            decimal: asset_type.config().decimals(),
        }
    }

    pub fn value(&self) -> u128 {
        self.value
    }

    pub fn asset_type(&self) -> AssetType {
        self.asset_type
    }

    pub fn validate_decimal(&self) -> anyhow::Result<()> {
        if self.asset_type.config().decimals() != self.decimal {
            anyhow::bail!(
                "Amount: Decimal not match, should be {} but got {}",
                self.asset_type.config().decimals(),
                self.decimal
            );
        }
        Ok(())
    }
}

impl std::convert::From<CkAmount> for ClientAmount {
    fn from(ck_amount: CkAmount) -> Self {
        Self {
            value: ck_amount.value(),
            asset_type: ck_amount.asset_type(),
            decimal: ck_amount.asset_type().config().decimals(),
        }
    }
}

impl std::convert::TryFrom<ClientAmount> for CkAmount {
    type Error = anyhow::Error;

    fn try_from(client_amount: ClientAmount) -> anyhow::Result<Self> {
        // ensure decimal match
        client_amount.validate_decimal()?;
        Ok(Self::new(client_amount.value, client_amount.asset_type))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClientFeeInfo {
    #[serde(with = "serde_util::f64_string")]
    pub fee_rate: f64,
    #[serde(with = "serde_util::u128_string")]
    pub units: u128,
    pub asset_type: AssetType,
    pub decimal: u32,
}

impl ClientFeeInfo {
    pub fn new(fee_rate: f64, units: u128, asset_type: AssetType) -> Self {
        Self {
            fee_rate,
            units,
            asset_type,
            decimal: asset_type.config().decimals(),
        }
    }

    pub fn validate_decimal(&self) -> anyhow::Result<()> {
        if self.asset_type.config().decimals() != self.decimal {
            anyhow::bail!(
                "FeeInfo: Decimal not match, should be {} but got {}",
                self.asset_type.config().decimals(),
                self.decimal
            );
        }
        Ok(())
    }
}

impl std::convert::From<FeeInfo> for ClientFeeInfo {
    fn from(fee_info: FeeInfo) -> Self {
        Self {
            fee_rate: fee_info.fee_rate,
            units: fee_info.units,
            asset_type: fee_info.asset_type,
            decimal: fee_info.asset_type.config().decimals(),
        }
    }
}

impl std::convert::TryFrom<ClientFeeInfo> for FeeInfo {
    type Error = anyhow::Error;

    fn try_from(client_fee_info: ClientFeeInfo) -> anyhow::Result<Self> {
        // ensure decimal match
        client_fee_info.validate_decimal()?;
        Ok(Self {
            fee_rate: client_fee_info.fee_rate,
            units: client_fee_info.units,
            asset_type: client_fee_info.asset_type,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClientTxTransfer {
    pub from: WalletID,
    pub to: ClientExternalAddress,
    pub amount: ClientAmount,
    pub fee_info: ClientFeeInfo,
}

impl ClientTxTransfer {
    pub fn adjust_gas_price_to_actually_used(
        &mut self,
        actual_gas_price_eth: u128,
    ) -> anyhow::Result<()> {
        let actual_gas_price = u128_to_f64(actual_gas_price_eth)?;
        self.fee_info.fee_rate = actual_gas_price;
        Ok(())
    }
}

impl std::convert::From<CkTransferInfo> for ClientTxTransfer {
    fn from(ck_transfer_info: CkTransferInfo) -> Self {
        let amount = ClientAmount::from(ck_transfer_info.amount);
        Self {
            from: ck_transfer_info.from_wallet,
            to: ck_transfer_info.to.into(),
            amount,
            fee_info: ck_transfer_info.fee_info.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReversedClientTxTransfer {
    pub from: ClientExternalAddress,
    pub to: WalletID,
    pub amount: ClientAmount,
}

impl std::convert::From<CkReversedTransferInfo> for ReversedClientTxTransfer {
    fn from(ck_reversed_transfer_info: CkReversedTransferInfo) -> Self {
        let amount = ClientAmount::from(ck_reversed_transfer_info.amount);
        Self {
            from: ck_reversed_transfer_info.from.into(),
            to: ck_reversed_transfer_info.to_wallet,
            amount,
        }
    }
}
