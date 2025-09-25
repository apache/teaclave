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

use serde::{Deserialize, Serialize};
use types::share::{
    ApprovalOperation, CkHash, CkSignature, MultiChainTransaction, TaApprovalChain,
    TaOperatorsBasic, TransactionID, WalletID,
};

// Transaction cmds (tls):
// - CreateTransaction
// - ApproveTransaction
// - DeleteTransaction
// - SignTransaction
// - ListPendingTransaction

// CreateTransaction
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateTransactionInput {
    pub tx: MultiChainTransaction,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateTransactionOutput {
    pub tx_id: TransactionID,
    pub latest_approval_chain: TaApprovalChain,
}

// ApproveTransaction
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApproveTransactionInput {
    pub tx_id: TransactionID,
    pub current_approval_chain: TaApprovalChain,
    pub operation: ApprovalOperation,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApproveTransactionOutput {
    pub latest_approval_chain: TaApprovalChain,
}

// DeleteTransaction
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecallTransactionInput {
    pub tx_id: TransactionID,
    pub current_approval_chain: TaApprovalChain,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RecallTransactionOutput {
    pub tx_id: TransactionID,
}

// SignTransaction
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignTransactionInput {
    pub tx_id: TransactionID,
    pub tx: MultiChainTransaction,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignTransactionOutput {
    pub signed_tx: CkSignature,
}

// ListPendingTransaction
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListPendingTransactionInput {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaTransactionDetail {
    pub tx_id: TransactionID,
    pub tx_hash: CkHash,
    pub from_wallet: WalletID,
    pub latest_approval_chain: TaApprovalChain,
    pub authorized_operators: TaOperatorsBasic,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListPendingTransactionOutput {
    pub tx_details: Vec<TaTransactionDetail>,
}
