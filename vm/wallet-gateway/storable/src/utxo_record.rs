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
use types::share::ClientBtcAddress;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UtxoRecord {
    pub address: ClientBtcAddress,
    pub txid: String,
    pub vout: u64,
    pub value: u64,
    pub block_height: u32,
    pub block_hash: String,
    pub block_time: u64,
}

impl UtxoRecord {
    pub fn new(
        address: ClientBtcAddress,
        txid: &str,
        vout: u64,
        value: u64,
        block_height: u32,
        block_hash: &str,
        block_time: u64,
    ) -> Self {
        Self {
            address,
            txid: txid.to_owned(),
            vout,
            value,
            block_height,
            block_hash: block_hash.to_owned(),
            block_time,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddressUtxoRecords {
    address: ClientBtcAddress,
    utxos: Vec<UtxoRecord>,
}

impl AddressUtxoRecords {
    pub fn new(address: ClientBtcAddress) -> Self {
        Self {
            address,
            utxos: Vec::new(),
        }
    }

    pub fn push(&mut self, utxo: UtxoRecord) {
        self.utxos.push(utxo);
    }

    pub fn address(&self) -> &ClientBtcAddress {
        &self.address
    }

    pub fn utxos(&self) -> &Vec<UtxoRecord> {
        &self.utxos
    }

    pub fn total_balance(&self) -> u64 {
        self.utxos.iter().map(|utxo| utxo.value).sum()
    }
}

impl Storable<String> for AddressUtxoRecords {
    fn unique_id(&self) -> String {
        self.address.address_str().to_owned()
    }
}
