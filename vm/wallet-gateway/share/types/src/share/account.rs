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

use crate::share::AccountXpub;
use anyhow::{anyhow, Result};
use basic_utils::keccak_hash_to_bytes;
use bitcoin::secp256k1;
use serde::{Deserialize, Serialize};
use serde_hex::SerHex;
use serde_hex::StrictPfx;
use std::convert::TryFrom;
use std::hash::Hash;

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Clone)]
pub struct AccountId(#[serde(with = "SerHex::<StrictPfx>")] [u8; 20]);

impl AccountId {
    pub fn take(self) -> [u8; 20] {
        self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn as_hex(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }
}

impl From<&secp256k1::PublicKey> for AccountId {
    fn from(public_key: &secp256k1::PublicKey) -> Self {
        let uncompressed_public_key = &public_key.serialize_uncompressed()[1..];
        let hash = keccak_hash_to_bytes(uncompressed_public_key);
        let mut v = [0u8; 20];
        v.copy_from_slice(&hash[12..]);
        AccountId(v)
    }
}

impl std::fmt::Display for AccountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hex = hex::encode(self.0);
        write!(f, "0x{}", hex)
    }
}

impl std::convert::From<[u8; 20]> for AccountId {
    fn from(bytes: [u8; 20]) -> Self {
        AccountId(bytes)
    }
}

impl std::convert::From<AccountId> for String {
    fn from(account_id: AccountId) -> Self {
        account_id.to_string()
    }
}

impl std::convert::TryFrom<String> for AccountId {
    type Error = anyhow::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        let s = s.trim_start_matches("0x");
        let bytes = hex::decode(s)?;
        if bytes.len() != 20 {
            return Err(anyhow!("invalid account id"));
        }
        let mut v = [0u8; 20];
        v.copy_from_slice(&bytes);
        Ok(AccountId(v))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SerializedAccount(pub Vec<u8>);

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub enum MultiChainAccount {
    Eth(AccountXpub),
    Btc(AccountXpub),
}

impl MultiChainAccount {
    pub fn id(&self) -> AccountId {
        match self {
            MultiChainAccount::Eth(account) | MultiChainAccount::Btc(account) => {
                account.compute_id()
            }
        }
    }

    pub fn account_index(&self) -> u32 {
        match self {
            MultiChainAccount::Eth(account) | MultiChainAccount::Btc(account) => account.index(),
        }
    }
}

impl TryFrom<MultiChainAccount> for SerializedAccount {
    type Error = anyhow::Error;

    fn try_from(account: MultiChainAccount) -> Result<Self> {
        let bytes = bincode::serialize(&account)?;
        Ok(SerializedAccount(bytes))
    }
}

impl TryFrom<SerializedAccount> for MultiChainAccount {
    type Error = anyhow::Error;

    fn try_from(account: SerializedAccount) -> Result<Self> {
        let account = bincode::deserialize(&account.0)?;
        Ok(account)
    }
}

// when dispatch client transfer, we need to distinguish chain type based on asset type
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub enum MultiChainAccountId {
    Eth(AccountId),
    Btc(AccountId),
}

impl MultiChainAccountId {
    pub fn id(&self) -> &AccountId {
        match self {
            MultiChainAccountId::Eth(id) => id,
            MultiChainAccountId::Btc(id) => id,
        }
    }

    pub fn from_mca(mca: &MultiChainAccount) -> MultiChainAccountId {
        match mca {
            MultiChainAccount::Eth(account) => MultiChainAccountId::Eth(account.compute_id()),
            MultiChainAccount::Btc(account) => MultiChainAccountId::Btc(account.compute_id()),
        }
    }

    pub fn is_eth(&self) -> bool {
        matches!(self, MultiChainAccountId::Eth(_))
    }

    pub fn is_btc(&self) -> bool {
        matches!(self, MultiChainAccountId::Btc(_))
    }
}
