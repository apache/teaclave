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

use crate::serde_util;
use crate::share::{AccountId, CkHash, CkHasher, WalletID};
use anyhow::{anyhow, Result};
use ethereum_tx_sign::LegacyTransaction;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EthTransaction {
    pub chain: u64,
    #[serde(with = "serde_util::option_u128_hex")]
    pub nonce: Option<u128>, // CK will adjust the nonce for final signing
    pub from_wallet: WalletID,
    pub from_account: AccountId, // erc20 token from
    pub to: [u8; 20],            // erc20 token to field is a contract address, when is optional?
    #[serde(with = "serde_util::u128_hex")]
    pub value: u128, // 0 for erc20 token transfer
    #[serde(with = "serde_util::u128_hex")]
    pub gas_price: u128,
    #[serde(with = "serde_util::u128_hex")]
    pub gas: u128,
    pub data: Vec<u8>,
}

impl EthTransaction {
    pub fn from_account(&self) -> &AccountId {
        &self.from_account
    }

    pub fn from_wallet(&self) -> &WalletID {
        &self.from_wallet
    }
}

impl CkHasher for EthTransaction {
    fn hash(&self) -> Result<CkHash> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.chain.to_be_bytes());
        bytes.extend_from_slice(self.from_account.as_bytes());
        bytes.extend_from_slice(&self.to);
        bytes.extend_from_slice(&self.value.to_be_bytes());
        bytes.extend_from_slice(&self.data);

        CkHash::new(bytes)
    }
}

impl TryFrom<EthTransaction> for LegacyTransaction {
    type Error = anyhow::Error;

    fn try_from(tx: EthTransaction) -> Result<Self> {
        Ok(LegacyTransaction {
            chain: tx.chain,
            nonce: tx.nonce.ok_or(anyhow!("nonce is required"))?,
            to: Some(tx.to), // Recipient (None when contract creation)
            gas_price: tx.gas_price,
            gas: tx.gas,
            value: tx.value,
            data: tx.data,
        })
    }
}
