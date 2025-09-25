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

pub mod btc;
pub mod eth;

use std::collections::HashSet;

use crate::external::{BtcAccount, EthAccount};
use crate::share::{
    AccountId, BtcAddress, ClientBtcAddress, EthAddress, MultiChainAccount, NetworkType,
};
use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CkAccount {
    Eth(EthAccount),
    Btc(BtcAccount),
}

impl CkAccount {
    pub fn id(&self) -> AccountId {
        match self {
            CkAccount::Eth(account) => account.id(),
            CkAccount::Btc(account) => account.id(),
        }
    }

    pub fn invoice_address_string(&self) -> Result<String> {
        match self {
            CkAccount::Eth(account) => Ok(account.invoice_address()?.to_string()),
            CkAccount::Btc(account) => Ok(account.invoice_address()?.to_string()),
        }
    }

    pub fn take_eth_account(self) -> Option<EthAccount> {
        match self {
            CkAccount::Eth(account) => Some(account),
            _ => None,
        }
    }

    pub fn take_btc_account(self) -> Option<BtcAccount> {
        match self {
            CkAccount::Btc(account) => Some(account),
            _ => None,
        }
    }

    pub fn all_client_btc_addresses(&self) -> HashSet<ClientBtcAddress> {
        match self {
            CkAccount::Btc(account) => account.all_client_addresses(),
            _ => HashSet::new(),
        }
    }

    pub fn all_btc_addresses(&self) -> Result<Vec<BtcAddress>> {
        match self {
            CkAccount::Btc(account) => account.all_addresses(),
            _ => Ok(Vec::new()),
        }
    }

    pub fn btc_change_address(&self) -> Result<BtcAddress> {
        match self {
            CkAccount::Btc(account) => account.current_change_address(),
            _ => Err(anyhow::anyhow!("Account is not BTC")),
        }
    }
}

impl std::convert::TryFrom<MultiChainAccount> for CkAccount {
    type Error = anyhow::Error;

    fn try_from(mca: MultiChainAccount) -> Result<Self> {
        match mca {
            MultiChainAccount::Eth(account) => {
                let eth_account = EthAccount::new(account);
                Ok(Self::Eth(eth_account))
            }
            MultiChainAccount::Btc(account) => {
                let btc_account = BtcAccount::try_init(account)?;
                Ok(Self::Btc(btc_account))
            }
        }
    }
}
