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

pub mod btc;
pub mod eth;
pub mod multichain;

use anyhow::{anyhow, ensure, Result};
use basic_utils::generate_random_bytes;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::hash::Hash;
use uuid::Uuid;

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Debug, Clone)]
pub struct TransactionID(pub Uuid);

impl TransactionID {
    pub fn new() -> Result<Self> {
        let random_bytes = generate_random_bytes(16)?;
        let uuid = uuid::Builder::from_random_bytes(
            random_bytes
                .try_into()
                .map_err(|_| anyhow!("[-] TransactionID::new(): invalid random bytes"))?,
        )
        .into_uuid();
        Ok(Self(uuid))
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.as_bytes().to_vec()
    }
}

impl From<Vec<u8>> for TransactionID {
    fn from(v: Vec<u8>) -> Self {
        Self(Uuid::from_slice(&v).unwrap())
    }
}

impl TryFrom<String> for TransactionID {
    type Error = anyhow::Error;

    fn try_from(src: String) -> Result<Self> {
        // check wallet id format (uuid)
        let parts: Vec<&str> = src.split('-').collect();
        ensure!(
            parts.len() == 5
                && parts[0].len() == 8
                && parts[1].len() == 4
                && parts[2].len() == 4
                && parts[3].len() == 4
                && parts[4].len() == 12,
            "invalid tx id format, should be XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX"
        );
        let id = Uuid::parse_str(&src).map_err(|_| anyhow!("invalid tx id"))?;
        Ok(TransactionID(id))
    }
}

impl From<TransactionID> for String {
    fn from(id: TransactionID) -> Self {
        id.0.to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TransactionStatus {
    PendingForApproval,
    Approved,
    Rejected,
}
