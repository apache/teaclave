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

use crate::external::AssetType;
use anyhow::Result;
use bitcoin::address::NetworkUnchecked;
use serde::{Deserialize, Serialize};
use serde_hex::{SerHex, StrictPfx};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "recipientType", content = "address")]
pub enum ExternalAddress {
    Eth(#[serde(with = "SerHex::<StrictPfx>")] [u8; 20]),
    Btc(String),
}

impl ExternalAddress {
    pub fn try_from_eth(hex: &str) -> Result<Self> {
        let addr = Self::check_evm_address(hex);
        Ok(Self::Eth(addr))
    }

    fn check_evm_address(addr: &str) -> [u8; 20] {
        // strip "0x" prefix if exist
        let hex = addr.strip_prefix("0x").unwrap_or(addr);
        let bytes = hex::decode(hex).unwrap();
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&bytes);
        addr
    }

    pub fn try_from_btc(addr: &str) -> Result<Self> {
        let address: bitcoin::Address<NetworkUnchecked> = addr
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid bitcoin address: {}", e))?;
        if address.is_valid_for_network(bitcoin::Network::Testnet)
            | address.is_valid_for_network(bitcoin::Network::Bitcoin)
        {
            Ok(Self::Btc(addr.into()))
        } else {
            anyhow::bail!("invalid btc address")
        }
    }

    pub fn is_eth(&self) -> bool {
        matches!(self, ExternalAddress::Eth(_))
    }

    pub fn is_btc(&self) -> bool {
        matches!(self, ExternalAddress::Btc(_))
    }

    pub fn try_take_eth(self) -> Result<[u8; 20]> {
        match self {
            ExternalAddress::Eth(addr) => Ok(addr),
            _ => anyhow::bail!("not eth address"),
        }
    }

    pub fn try_take_btc(self) -> Result<String> {
        match self {
            ExternalAddress::Btc(addr) => Ok(addr),
            _ => anyhow::bail!("not btc address"),
        }
    }
}

impl std::fmt::Display for ExternalAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExternalAddress::Eth(addr) => write!(f, "0x{}", hex::encode(addr)),
            ExternalAddress::Btc(addr) => write!(f, "{}", addr),
        }
    }
}

// dependency of "storable" and "types-client"
#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct ClientExternalAddress(pub String);
// length check
impl<'de> serde::de::Deserialize<'de> for ClientExternalAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let value_str: &str = Deserialize::deserialize(deserializer)?;
        if value_str.len() > 62 {
            // bitcoin taproot address length
            return Err(serde::de::Error::custom(
                "ClientExternalAddress length exceeded",
            ));
        }
        if value_str.is_empty() {
            return Err(serde::de::Error::custom("ClientExternalAddress empty"));
        }
        Ok(ClientExternalAddress(value_str.to_string()))
    }
}

impl ClientExternalAddress {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn to_external_address(&self, asset_type: &AssetType) -> Result<ExternalAddress> {
        if asset_type.is_evm_compatible() {
            ExternalAddress::try_from_eth(self.as_str())
        } else if asset_type.is_bitcoin_chain() {
            ExternalAddress::try_from_btc(self.as_str())
        } else {
            anyhow::bail!("'to' address: unsupported asset type: {:?}", asset_type)
        }
    }
}

impl std::convert::From<ExternalAddress> for ClientExternalAddress {
    fn from(external_address: ExternalAddress) -> Self {
        Self(external_address.to_string())
    }
}

impl std::fmt::Display for ClientExternalAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
