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

use crate::{AdditionalInfoForClient, ApprovalStageInfo, ClientTxTransfer};
use serde::{Deserialize, Serialize};
use storable::legacy_tx::Transaction;
use storable::tx_history::{HistoryTx, TxHistoryStatus};
use types::external::Email;
use types::share::{TransactionID, TransactionStatus};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingTxInfo {
    id: TransactionID,
    operator: Email,
    tx_transfer: ClientTxTransfer,
    approval_chain: Vec<ApprovalStageInfo>,
    created_at: u64,
    from_info: AdditionalInfoForClient,
    to_info: AdditionalInfoForClient,
}

impl PendingTxInfo {
    pub fn new(
        tx: Transaction,
        from_info: AdditionalInfoForClient,
        to_info: AdditionalInfoForClient,
    ) -> Self {
        let approval_chain_info: Vec<ApprovalStageInfo> = tx
            .approval_chain
            .into_iter()
            .map(|stage| stage.into())
            .collect();

        Self {
            id: tx.id,
            operator: tx.operator,
            tx_transfer: ClientTxTransfer::from(tx.transfer_info),
            approval_chain: approval_chain_info,
            created_at: tx.created_at,
            from_info,
            to_info,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTxInfo {
    id: TransactionID,
    operator: Email,
    tx_transfer: ClientTxTransfer,
    approval_chain: Vec<ApprovalStageInfo>,
    approval_chain_overall_status: TransactionStatus,
    created_at: u64,
    from_info: AdditionalInfoForClient,
    to_info: AdditionalInfoForClient,
    tx_submitted_info: TxSubmittedInfo,
}

impl HistoryTxInfo {
    pub fn new(
        tx_history: HistoryTx,
        explorer_base_url: url::Url,
        from_info: AdditionalInfoForClient,
        to_info: AdditionalInfoForClient,
    ) -> anyhow::Result<Self> {
        let tx = tx_history.tx;
        let overall_status = tx.overall_status();
        let asset_type = tx.transfer_info().asset_type();
        let gas_price_for_sign = tx.get_gas_price();
        let approval_chain = tx
            .approval_chain
            .into_iter()
            .map(|stage| stage.into())
            .collect();
        let history_status = tx_history.tx_status;
        let mut client_tx_transfer: ClientTxTransfer = ClientTxTransfer::from(tx.transfer_info);

        // if eth transaction, adjust fee rate to the actual used one
        if asset_type.is_evm_compatible() {
            client_tx_transfer.adjust_gas_price_to_actually_used(gas_price_for_sign)?;
        }

        Ok(Self {
            id: tx.id,
            operator: tx.operator,
            tx_transfer: client_tx_transfer,
            approval_chain,
            approval_chain_overall_status: overall_status,
            created_at: tx.created_at,
            from_info,
            to_info,
            tx_submitted_info: TxSubmittedInfo::new(
                history_status,
                &explorer_base_url,
                tx_history.timestamp,
            ),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "status")]
enum TxSubmittedInfo {
    OnChain(OnChainInfo),
    RejectedByNetwork(ErrorInfo),
    RejectedByApprover(OperationInfo),
    Cached(ErrorInfo),
    RecalledByOperator(OperationInfo),
    ReceivedPayment(OnChainInfo),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OnChainInfo {
    pub url: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ErrorInfo {
    pub error_message: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OperationInfo {
    pub timestamp: u64,
}

impl TxSubmittedInfo {
    pub fn new(status: TxHistoryStatus, explorer_base_url: &url::Url, timestamp: u64) -> Self {
        match status {
            TxHistoryStatus::OnChain(tx_hash) => {
                let url = url::Url::parse(&format!(
                    "{}tx/{}",
                    explorer_base_url.clone(),
                    tx_hash.as_str()
                ))
                .unwrap_or_else(|_| explorer_base_url.clone());
                TxSubmittedInfo::OnChain(OnChainInfo {
                    url: url.into(),
                    timestamp,
                })
            }
            TxHistoryStatus::RejectedByNetwork(msg) => {
                TxSubmittedInfo::RejectedByNetwork(ErrorInfo {
                    error_message: format!("{:?}", msg),
                    timestamp,
                })
            }
            TxHistoryStatus::RejectedByApprover => {
                TxSubmittedInfo::RejectedByApprover(OperationInfo { timestamp })
            }
            TxHistoryStatus::Cached => {
                let err_msg =
                    "Cached (network error or unknown error), please contact administator"
                        .to_owned();
                TxSubmittedInfo::Cached(ErrorInfo {
                    error_message: err_msg,
                    timestamp,
                })
            }
            TxHistoryStatus::RecalledByOperator => {
                TxSubmittedInfo::RecalledByOperator(OperationInfo { timestamp })
            }
        }
    }
}
