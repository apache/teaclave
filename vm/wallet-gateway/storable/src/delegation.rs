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

use crate::{legacy_tx::Transaction, Storable};
use serde::{Deserialize, Serialize};
use types::share::{CkSignature, TransactionID};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DelegationInfo {
    pub valid_duration: u64, // in seconds
    pub gas_unit_price: u128,
    pub gas_units: u128,
}

impl DelegationInfo {
    pub fn new(valid_duration: u64, gas_unit_price: u128, gas_units: u128) -> Self {
        Self {
            valid_duration,
            gas_unit_price,
            gas_units,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TxDelegation {
    pub tx: Transaction,
    pub delegation_info: DelegationInfo,
    pub signed_payload: Option<CkSignature>,
    pub expiration_time: u64,
}

impl TxDelegation {
    pub fn new(tx: Transaction, delegation_info: DelegationInfo) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expiration_time = now + delegation_info.valid_duration;
        Self {
            tx,
            delegation_info,
            expiration_time,
            signed_payload: None,
        }
    }

    pub fn tx(&self) -> &Transaction {
        &self.tx
    }

    pub fn take_tx(&self) -> Transaction {
        self.tx.clone()
    }

    pub fn expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.expiration_time
    }
}

impl Storable<TransactionID> for TxDelegation {
    fn unique_id(&self) -> TransactionID {
        self.tx.get_id()
    }
}
