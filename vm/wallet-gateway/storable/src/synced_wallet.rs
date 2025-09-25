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
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use types::external::{ApprovalChainBasic, Email, OperatorsBasic};
use types::share::{AccountId, MultiChainAccountId, WalletID};

// SyncedWalletInfo: Wallets synced from TEE
// ConsoleWalletInfo: Wallets created/modified by users
// ConsoleWalletInfo (authority) => TEE => SyncedWalletInfo (webapi)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncedWalletInfo {
    pub id: WalletID,
    pub approval_chain: ApprovalChainBasic,
    pub authorized_operators: OperatorsBasic,
    pub accounts: Vec<MultiChainAccountId>,
    pub config_version: u64,
}

impl Storable<WalletID> for SyncedWalletInfo {
    fn unique_id(&self) -> WalletID {
        self.id.clone()
    }
}

impl SyncedWalletInfo {
    pub fn new(
        id: WalletID,
        approval_chain: ApprovalChainBasic,
        authorized_operators: OperatorsBasic,
        accounts: Vec<MultiChainAccountId>,
        config_version: u64,
    ) -> Self {
        Self {
            id,
            approval_chain,
            authorized_operators,
            accounts,
            config_version,
        }
    }

    pub fn id(&self) -> &WalletID {
        &self.id
    }

    pub fn approval_chain(&self) -> &ApprovalChainBasic {
        &self.approval_chain
    }

    pub fn associated_with_user(&self, user: &Email) -> bool {
        self.is_authorized_operator(user) || self.approval_chain.contains(user)
    }

    pub fn associated_users(&self) -> HashSet<&Email> {
        let mut users = HashSet::new();
        users.extend(self.approval_chain.distinct_approvers());
        users.extend(self.authorized_operators.iter());
        users
    }

    pub fn associated_users_owned(&self) -> HashSet<Email> {
        let mut users = HashSet::new();
        users.extend(self.approval_chain.distinct_approvers());
        users.extend(self.authorized_operators.iter());
        users.into_iter().cloned().collect()
    }

    pub fn is_authorized_operator(&self, user: &Email) -> bool {
        self.authorized_operators.iter().any(|op| op == user)
    }

    pub fn accounts(&self) -> &Vec<MultiChainAccountId> {
        &self.accounts
    }

    pub fn iter_account(&self) -> impl Iterator<Item = &MultiChainAccountId> {
        self.accounts.iter()
    }

    pub fn btc_account(&self) -> Option<AccountId> {
        self.accounts.iter().find_map(|id| match id {
            MultiChainAccountId::Btc(id) => Some(id.clone()),
            _ => None,
        })
    }

    pub fn eth_account(&self) -> Option<AccountId> {
        self.accounts.iter().find_map(|id| match id {
            MultiChainAccountId::Eth(id) => Some(id.clone()),
            _ => None,
        })
    }
}
