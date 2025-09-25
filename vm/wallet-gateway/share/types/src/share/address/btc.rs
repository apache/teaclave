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

use std::{convert::TryInto, str::FromStr};

use crate::share::{CkNetwork, NetworkType};
use anyhow::{bail, Result};
use bitcoin::bip32::DerivationPath;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BtcAddress {
    inner: bitcoin::Address,
    network: NetworkType,
    path: DerivationPath,
}

impl BtcAddress {
    pub fn try_from_pk(
        pk: bitcoin::PublicKey,
        path: DerivationPath,
        network: bitcoin::Network,
    ) -> Result<Self> {
        let address = bitcoin::Address::p2wpkh(&pk, network)
            .map_err(|e| anyhow::anyhow!("create address failed: {}", e))?;
        let nt = match network {
            bitcoin::Network::Bitcoin => NetworkType::Mainnet,
            bitcoin::Network::Testnet => NetworkType::Testnet,
            _ => bail!("unsupported network"),
        };
        Ok(Self {
            inner: address,
            path,
            network: nt,
        })
    }

    pub fn network(&self) -> NetworkType {
        self.network
    }

    pub fn path(&self) -> &DerivationPath {
        &self.path
    }
}

impl std::fmt::Display for BtcAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl std::ops::Deref for BtcAddress {
    type Target = bitcoin::Address;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClientBtcAddress {
    // verify format through constructor
    address: String,
    network: NetworkType,
}

impl ClientBtcAddress {
    pub fn try_from(address: impl Into<String>, network: NetworkType) -> Result<ClientBtcAddress> {
        let string = address.into();
        let btc_network = CkNetwork::Btc(network).try_into()?;
        let _ = bitcoin::Address::from_str(string.as_str())
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .require_network(btc_network)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        Ok(ClientBtcAddress {
            address: string,
            network,
        })
    }

    pub fn address_str(&self) -> &str {
        &self.address
    }

    pub fn address(&self) -> String {
        self.address.clone()
    }

    pub fn network(&self) -> NetworkType {
        self.network
    }
}

impl std::convert::From<BtcAddress> for ClientBtcAddress {
    fn from(addr: BtcAddress) -> Self {
        Self {
            address: addr.to_string(),
            network: addr.network(),
        }
    }
}

impl std::convert::From<&BtcAddress> for ClientBtcAddress {
    fn from(addr: &BtcAddress) -> Self {
        Self {
            address: addr.to_string(),
            network: addr.network(),
        }
    }
}

impl std::convert::TryFrom<ClientBtcAddress> for bitcoin::Address {
    type Error = anyhow::Error;

    fn try_from(addr: ClientBtcAddress) -> Result<Self, Self::Error> {
        let btc_network = CkNetwork::Btc(addr.network).try_into()?;
        let address = bitcoin::Address::from_str(&addr.address)
            .map_err(|e| anyhow::anyhow!("{}", e))?
            .require_network(btc_network)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(address)
    }
}
