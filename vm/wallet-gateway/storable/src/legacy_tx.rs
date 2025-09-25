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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use types::external::{ApprovalChain, CkTransferInfo, Email};
use types::share::{MultiChainTransaction, TaApprovalChain, TransactionID, TransactionStatus};

use crate::address_book::AddressBookEntry;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    pub id: TransactionID,
    pub operator: Email,
    pub transfer_info: CkTransferInfo,
    pub tx: MultiChainTransaction,
    pub approval_chain: ApprovalChain,
    pub created_at: u64,
    pub address_name: AddressBookEntry,
}

impl Transaction {
    pub fn create(
        id: TransactionID,
        operator: Email,
        transfer_info: CkTransferInfo,
        tx: MultiChainTransaction,
        approval_chain: ApprovalChain,
        address_name: AddressBookEntry,
    ) -> Result<Self> {
        // current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        Ok(Self {
            id,
            operator,
            transfer_info,
            tx,
            approval_chain,
            created_at: timestamp,
            address_name,
        })
    }

    pub fn transfer_info(&self) -> &CkTransferInfo {
        &self.transfer_info
    }

    pub fn get_id(&self) -> TransactionID {
        self.id.clone()
    }

    pub fn get_operator(&self) -> &Email {
        &self.operator
    }

    pub fn update_approval_chain(&mut self, ta_chain: TaApprovalChain) -> Result<()> {
        let uid_to_email = self.approval_chain.uid_email_mapping();
        self.approval_chain.try_update(ta_chain, &uid_to_email)
    }

    pub fn current_stage_index(&self) -> Option<usize> {
        self.approval_chain.current_stage_index()
    }

    pub fn ready_for_current_stage(&self, user: &Email) -> bool {
        self.approval_chain.ready_for_current_stage(user)
    }

    pub fn current_stage_approvers(&self) -> Option<HashSet<Email>> {
        // get current stage
        self.current_stage_index()
            .map(|s| self.approval_chain.approvers_on_stage(s))
    }

    // get the approvers who approved/rejected the transaction
    pub fn get_operated_approvers(&self) -> HashSet<Email> {
        self.approval_chain.get_operated_approvers()
    }

    pub fn get_all_approvers(&self) -> HashSet<Email> {
        self.approval_chain.approvers()
    }

    pub fn get_approval_chain(&self) -> &ApprovalChain {
        &self.approval_chain
    }

    pub fn overall_status(&self) -> TransactionStatus {
        if self.approval_chain.any_reject() {
            TransactionStatus::Rejected
        } else if self.approval_chain.any_pending() {
            TransactionStatus::PendingForApproval
        } else {
            assert!(
                self.approval_chain.all_approved(),
                "Fatal: invalid approval chain"
            );
            TransactionStatus::Approved
        }
    }

    pub fn get_created_at(&self) -> u64 {
        self.created_at
    }

    pub fn associated_with_user(&self, user: &Email) -> bool {
        (&self.operator == user) || self.approval_chain.iter().any(|s| s.contains(user))
    }

    // BTC transaction doesn't need to set gas price
    pub fn get_gas_price(&self) -> u128 {
        match &self.tx {
            MultiChainTransaction::Eth(tx) => tx.gas_price,
            MultiChainTransaction::Btc(_) => 0,
        }
    }
    pub fn get_gas_limit(&self) -> u128 {
        match &self.tx {
            MultiChainTransaction::Eth(tx) => tx.gas,
            MultiChainTransaction::Btc(_) => 0,
        }
    }
    pub fn set_gas_price(&mut self, gas_price: u128) {
        match &mut self.tx {
            MultiChainTransaction::Eth(tx) => tx.gas_price = gas_price,
            MultiChainTransaction::Btc(_) => unimplemented!(),
        }
    }
    pub fn set_gas_limit(&mut self, gas_limit: u128) {
        match &mut self.tx {
            MultiChainTransaction::Eth(tx) => tx.gas = gas_limit,
            MultiChainTransaction::Btc(_) => unimplemented!(),
        }
    }

    pub fn get_multichain_tx_mut(&mut self) -> &mut MultiChainTransaction {
        &mut self.tx
    }
}
