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

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CkNetwork {
    Eth(NetworkType),
    Btc(NetworkType),
    Bsc(NetworkType),
}

impl CkNetwork {
    pub fn all_supported_networks(network_type: NetworkType) -> Vec<CkNetwork> {
        vec![
            CkNetwork::Eth(network_type),
            CkNetwork::Btc(network_type),
            CkNetwork::Bsc(network_type),
        ]
    }

    pub fn network_type(&self) -> NetworkType {
        match self {
            CkNetwork::Eth(network_type) => *network_type,
            CkNetwork::Btc(network_type) => *network_type,
            CkNetwork::Bsc(network_type) => *network_type,
        }
    }

    pub fn chain_id(&self) -> u64 {
        match self {
            CkNetwork::Eth(network_type) => match network_type {
                NetworkType::Mainnet => 1,
                NetworkType::Testnet => 11155111, // for sepolia
            },
            CkNetwork::Btc(_network_type) => 0, // btc does not have chain id
            CkNetwork::Bsc(network_type) => match network_type {
                NetworkType::Mainnet => 56,
                NetworkType::Testnet => 97,
            },
        }
    }

    // for sending tx, estimate gas, etc
    pub fn rpc_api_url(&self) -> Url {
        match self {
            CkNetwork::Eth(network_type) => match network_type {
                NetworkType::Mainnet => Url::parse("https://mainnet.infura.io/v3/").unwrap(), // we use unwrap() here because we are sure the url is valid
                NetworkType::Testnet => Url::parse("https://sepolia.infura.io/v3/").unwrap(),
            },
            CkNetwork::Btc(network_type) => match network_type {
                NetworkType::Mainnet => Url::parse("https://blockstream.info/api/").unwrap(),
                NetworkType::Testnet => {
                    Url::parse("https://blockstream.info/testnet/api/").unwrap()
                }
            },
            CkNetwork::Bsc(network_type) => match network_type {
                NetworkType::Mainnet => Url::parse("https://bsc-dataseed.bnbchain.org/").unwrap(),
                NetworkType::Testnet => Url::parse("https://bsc-testnet.bnbchain.org/").unwrap(),
            },
        }
    }

    // for fetching tx, utxos, price, etc
    pub fn explorer_api_url(&self) -> Url {
        match self {
            CkNetwork::Eth(network_type) => match network_type {
                NetworkType::Mainnet => Url::parse("https://api.etherscan.io/api").unwrap(),
                NetworkType::Testnet => Url::parse("https://api-sepolia.etherscan.io/api").unwrap(),
            },
            CkNetwork::Btc(network_type) => match network_type {
                NetworkType::Mainnet => Url::parse("https://mempool.space/api/").unwrap(),
                NetworkType::Testnet => Url::parse("https://mempool.space/testnet/api").unwrap(),
            },
            CkNetwork::Bsc(network_type) => match network_type {
                NetworkType::Mainnet => Url::parse("https://api.bscscan.com/api/").unwrap(),
                NetworkType::Testnet => Url::parse("https://api-testnet.bscscan.com/api").unwrap(),
            },
        }
    }

    // for show external links in email or web
    pub fn explorer_base_url(&self) -> Url {
        match self {
            CkNetwork::Eth(network_type) => match network_type {
                NetworkType::Mainnet => Url::parse("https://etherscan.io/").unwrap(),
                NetworkType::Testnet => Url::parse("https://sepolia.etherscan.io/").unwrap(),
            },
            CkNetwork::Btc(network_type) => match network_type {
                NetworkType::Mainnet => Url::parse("https://mempool.space/").unwrap(),
                NetworkType::Testnet => Url::parse("https://mempool.space/testnet/").unwrap(),
            },
            CkNetwork::Bsc(network_type) => match network_type {
                NetworkType::Mainnet => Url::parse("https://bscscan.com/").unwrap(),
                NetworkType::Testnet => Url::parse("https://testnet.bscscan.com/").unwrap(),
            },
        }
    }
}

impl std::fmt::Display for CkNetwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CkNetwork::Eth(network) => write!(f, "eth-{}", network),
            CkNetwork::Btc(network) => write!(f, "btc-{}", network),
            CkNetwork::Bsc(network) => write!(f, "bsc-{}", network),
        }
    }
}

impl std::convert::TryFrom<String> for CkNetwork {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 2 {
            return Err(anyhow!("invalid network"));
        }
        let network_type = match parts[1] {
            "mainnet" => NetworkType::Mainnet,
            "testnet" => NetworkType::Testnet,
            _ => return Err(anyhow!("invalid network")),
        };
        match parts[0] {
            "eth" => Ok(CkNetwork::Eth(network_type)),
            "btc" => Ok(CkNetwork::Btc(network_type)),
            "bsc" => Ok(CkNetwork::Bsc(network_type)),
            _ => Err(anyhow!("invalid network")),
        }
    }
}

impl std::convert::From<CkNetwork> for String {
    fn from(network: CkNetwork) -> Self {
        format!("{}", network)
    }
}

impl std::convert::TryFrom<CkNetwork> for bitcoin::Network {
    type Error = anyhow::Error;

    fn try_from(network: CkNetwork) -> Result<Self, Self::Error> {
        if let CkNetwork::Btc(network_type) = network {
            match network_type {
                NetworkType::Mainnet => Ok(bitcoin::Network::Bitcoin),
                NetworkType::Testnet => Ok(bitcoin::Network::Testnet),
            }
        } else {
            Err(anyhow!("unsupported network"))
        }
    }
}

impl serde::ser::Serialize for CkNetwork {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        String::from(*self).serialize(serializer)
    }
}

impl<'de> serde::de::Deserialize<'de> for CkNetwork {
    fn deserialize<D>(deserializer: D) -> Result<CkNetwork, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        CkNetwork::try_from(s).map_err(serde::de::Error::custom)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetworkType {
    Mainnet,
    Testnet,
}

impl std::fmt::Display for NetworkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkType::Mainnet => write!(f, "mainnet"),
            NetworkType::Testnet => write!(f, "testnet"),
        }
    }
}

impl TryFrom<bitcoin::Network> for NetworkType {
    type Error = anyhow::Error;

    fn try_from(network: bitcoin::Network) -> Result<Self> {
        match network {
            bitcoin::Network::Bitcoin => Ok(NetworkType::Mainnet),
            bitcoin::Network::Testnet => Ok(NetworkType::Testnet),
            _ => Err(anyhow!("unsupported network: {:?}", network)),
        }
    }
}

impl From<NetworkType> for bitcoin::Network {
    fn from(network: NetworkType) -> Self {
        match network {
            NetworkType::Mainnet => bitcoin::Network::Bitcoin,
            NetworkType::Testnet => bitcoin::Network::Testnet,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChainType {
    Eth,
    Btc,
}

// used by TA add_account()
pub const SUPPORTED_CHAIN_TYPES: [ChainType; 2] = [ChainType::Eth, ChainType::Btc];
