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

use crate::{AdditionalInfoForClient, ReversedClientTxTransfer};
use serde::{Deserialize, Serialize};
// todo: fix
use storable::received_payment::PaymentRecord;
use types::share::{NetworkType, WalletID};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedPaymentInfo {
    #[serde(flatten)]
    transfer_info: ReversedClientTxTransfer,
}
impl ReceivedPaymentInfo {
    pub fn new(payment_record: PaymentRecord) -> Self {
        Self {
            transfer_info: payment_record.transfer_info.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExplorerInfo {
    url: url::Url,
    timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxHistoryReceivedPayment {
    pub tx_transfer: ReceivedPaymentInfo,
    pub from_info: AdditionalInfoForClient,
    pub to_info: AdditionalInfoForClient,
    pub explorer_info: ExplorerInfo,
}
impl TxHistoryReceivedPayment {
    pub fn new(
        payment: PaymentRecord,
        network_type: NetworkType,
        from_info: AdditionalInfoForClient,
        to_info: AdditionalInfoForClient,
    ) -> Self {
        let tx_hash = payment.tx_hash.clone();
        let ck_network = payment
            .transfer_info
            .amount
            .asset_type()
            .as_ck_network(network_type);
        let explorer_base_url = ck_network.explorer_base_url();
        let explorer_url = explorer_base_url
            .join(&format!("tx/{}", tx_hash))
            .unwrap_or(explorer_base_url);
        let timestamp = payment.timestamp;
        let tx_transfer_received_payment: ReceivedPaymentInfo = ReceivedPaymentInfo::new(payment);
        Self {
            tx_transfer: tx_transfer_received_payment,
            from_info,
            to_info,
            explorer_info: ExplorerInfo {
                url: explorer_url,
                timestamp,
            },
        }
    }

    pub fn to_wallet(&self) -> WalletID {
        self.tx_transfer.transfer_info.to.clone()
    }
}
