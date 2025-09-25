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

use std::collections::HashMap;

use crate::legacy_tx::Transaction;
use crate::Storable;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use types::{
    external::{AssetType, CkAmount},
    share::TransactionID,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "status")]
pub enum PendingTx {
    PendingForApproval(Transaction),
    ReadyForSigning(Transaction),
    DelegationStarted(Transaction), // indicates there is a copy in the TxDelegation table
    DelegationExpired(Transaction), // all attempts failed, indicate TxDelegation is removed, single copy of PendingTx
}

impl PendingTx {
    pub fn tx(&self) -> &Transaction {
        match self {
            PendingTx::PendingForApproval(tx) => tx,
            PendingTx::ReadyForSigning(tx) => tx,
            PendingTx::DelegationStarted(tx) => tx,
            PendingTx::DelegationExpired(tx) => tx,
        }
    }

    // for error log
    pub fn status(&self) -> &str {
        match self {
            PendingTx::PendingForApproval(_) => "pendingForApproval",
            PendingTx::ReadyForSigning(_) => "readyForSigning",
            PendingTx::DelegationStarted(_) => "delegationStarted",
            PendingTx::DelegationExpired(_) => "DelegationExpired",
        }
    }

    pub fn total_spend(&self) -> Result<HashMap<AssetType, CkAmount>> {
        self.tx().transfer_info.total_spend()
    }

    pub fn asset_type(&self) -> AssetType {
        self.tx().transfer_info.asset_type()
    }
}

impl Storable<TransactionID> for PendingTx {
    fn table_name() -> &'static str {
        "PendingTx"
    }
    fn unique_id(&self) -> TransactionID {
        self.tx().get_id()
    }
}
