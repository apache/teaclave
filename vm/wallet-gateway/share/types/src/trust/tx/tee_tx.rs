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

use crate::{
    share::{CkHash, TaApprovalChain, TaOperatorsBasic, TransactionID, UserID, WalletID},
    Storable,
};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TeeTransaction {
    id: TransactionID,                      // random uuid from tee
    hash: CkHash,                           // MultiChainTx::hash(&self) -> CkHash::new(&self)
    from_wallet: WalletID,                  // created from which wallet
    approval_chain: TaApprovalChain,        // from tee, sync to external
    authorized_operators: TaOperatorsBasic, // from external wallet_info
}

impl Storable<TransactionID> for TeeTransaction {
    fn unique_id(&self) -> TransactionID {
        self.id.clone()
    }
}

impl TeeTransaction {
    pub fn new(
        hash: CkHash,
        from_wallet: WalletID,
        // creator: UserID,
        approval_chain: TaApprovalChain,
        authorized_operators: TaOperatorsBasic,
    ) -> Result<Self> {
        Ok(Self {
            id: TransactionID::new()?,
            hash,
            from_wallet,
            // creator,
            approval_chain,
            authorized_operators,
        })
    }

    pub fn id(&self) -> &TransactionID {
        &self.id
    }

    pub fn hash(&self) -> &CkHash {
        &self.hash
    }

    pub fn from_wallet(&self) -> &WalletID {
        &self.from_wallet
    }

    pub fn authorized_operators(&self) -> &TaOperatorsBasic {
        &self.authorized_operators
    }

    pub fn approve(
        &mut self,
        user_id: &UserID,
        current_approval_chain: &TaApprovalChain,
    ) -> Result<()> {
        ensure!(
            self.is_approval_chain_up_to_date(current_approval_chain),
            "approval chain is not up to date"
        );
        self.approval_chain.approve(user_id)
    }

    pub fn reject(
        &mut self,
        user_id: &UserID,
        current_approval_chain: &TaApprovalChain,
    ) -> Result<()> {
        ensure!(
            self.is_approval_chain_up_to_date(current_approval_chain),
            "approval chain is not up to date"
        );
        self.approval_chain.reject(user_id)
    }

    pub fn is_ready_for_sign(&self) -> bool {
        self.approval_chain.all_approved()
    }

    pub fn hash_matches(&self, hash: &CkHash) -> bool {
        self.hash == *hash
    }

    pub fn is_approval_chain_up_to_date(&self, current_approval_chain: &TaApprovalChain) -> bool {
        self.approval_chain.match_other(current_approval_chain)
    }

    pub fn get_approval_chain(&self) -> &TaApprovalChain {
        &self.approval_chain
    }
}
