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
use types::external::CkReversedTransferInfo;
use types::share::{AccountId, CkNetwork};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInfo {
    pub account: AccountId,           // address as index
    pub ck_network: CkNetwork,        // network type for the payments
    pub payments: Vec<PaymentRecord>, // save latest 20 records by timestamp desc
    pub last_notified_tx_block_number: u64,
}

impl PaymentInfo {
    pub fn new(account: AccountId, ck_network: CkNetwork) -> Self {
        Self {
            account,
            ck_network,
            payments: Vec::new(),
            last_notified_tx_block_number: 0,
        }
    }

    pub fn add_new_records(&mut self, records: Vec<PaymentRecord>) {
        if records.is_empty() {
            return;
        }
        let mut new_records = records.clone();
        new_records.extend(self.payments.clone());
        new_records.sort_by(|a, b| b.block_number.cmp(&a.block_number));

        self.last_notified_tx_block_number = new_records[0].block_number;
        self.payments = new_records;
    }

    pub fn get_records_owned(&self) -> Vec<PaymentRecord> {
        self.payments.clone()
    }
}

impl Storable<String> for PaymentInfo {
    fn unique_id(&self) -> String {
        // AccountId + CkNetwork
        format!("{}-{}", self.account, self.ck_network)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentRecord {
    pub transfer_info: CkReversedTransferInfo,
    pub timestamp: u64,
    pub tx_hash: String,
    pub block_number: u64,
}

impl PaymentRecord {
    pub fn new(
        transfer_info: CkReversedTransferInfo,
        timestamp: u64,
        tx_hash: String,
        block_number: u64,
    ) -> Self {
        Self {
            transfer_info,
            timestamp,
            tx_hash,
            block_number,
        }
    }

    pub fn try_add(&mut self, value: u128) -> anyhow::Result<()> {
        self.transfer_info.amount.try_add_u128(value)
    }
}
