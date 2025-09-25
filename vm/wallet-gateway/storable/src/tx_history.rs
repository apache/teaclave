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

use crate::legacy_tx::Transaction;
use crate::Storable;
use serde::{Deserialize, Serialize};
use types::external::{NetworkErrMsg, NetworkTxHash, TxSubmissionResult};
use types::share::{CkSignature, TransactionID};

type Timestamp = u64;
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryTx {
    pub tx: Transaction,
    pub signed_payload: Option<CkSignature>,
    pub tx_status: TxHistoryStatus,
    pub timestamp: Timestamp,
}

pub fn now_tms() -> Timestamp {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

impl HistoryTx {
    pub fn from_submitted(
        tx: Transaction,
        signed_payload: Option<CkSignature>,
        sr: TxSubmissionResult,
    ) -> Self {
        HistoryTx {
            tx,
            signed_payload,
            tx_status: match sr {
                TxSubmissionResult::Accepted(tx_hash) => TxHistoryStatus::OnChain(tx_hash),
                TxSubmissionResult::Rejected(err_msg) => {
                    TxHistoryStatus::RejectedByNetwork(err_msg)
                }
            },
            timestamp: now_tms(),
        }
    }

    pub fn from_approver_rejected(tx: Transaction) -> Self {
        HistoryTx {
            tx,
            signed_payload: None,
            tx_status: TxHistoryStatus::RejectedByApprover,
            timestamp: now_tms(),
        }
    }

    pub fn from_operator_recalled(tx: Transaction) -> Self {
        HistoryTx {
            tx,
            signed_payload: None,
            tx_status: TxHistoryStatus::RecalledByOperator,
            timestamp: now_tms(),
        }
    }

    pub fn tx(&self) -> &Transaction {
        &self.tx
    }

    pub fn tx_status(&self) -> &TxHistoryStatus {
        &self.tx_status
    }

    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    pub fn message(&self) -> String {
        match &self.tx_status {
            TxHistoryStatus::OnChain(tx_hash) => format!("tx hash: {:?}", tx_hash),
            TxHistoryStatus::RejectedByNetwork(msg) => {
                format!("error message: {:?}", msg)
            }
            TxHistoryStatus::RejectedByApprover => "rejected by approver".to_string(),
            TxHistoryStatus::Cached => "network error or unknown error".to_string(),
            TxHistoryStatus::RecalledByOperator => "recalled by operator".to_string(),
        }
    }
}

impl Storable<TransactionID> for HistoryTx {
    fn table_name() -> &'static str {
        "HistoryTx"
    }
    fn unique_id(&self) -> TransactionID {
        self.tx.get_id()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TxHistoryStatus {
    OnChain(NetworkTxHash),
    RejectedByNetwork(NetworkErrMsg),
    RejectedByApprover,
    Cached, // cannot connect to infura
    RecalledByOperator,
}
