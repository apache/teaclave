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

use anyhow::{anyhow, ensure, Result};
use basic_utils::generate_random_bytes;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::hash::Hash;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct WalletID(pub Uuid);

impl WalletID {
    pub fn new() -> Result<Self> {
        let random_bytes = generate_random_bytes(16)?;
        let uuid = uuid::Builder::from_random_bytes(
            random_bytes
                .try_into()
                .map_err(|_| anyhow!("[-] WalletID::new(): invalid random bytes"))?,
        )
        .into_uuid();
        Ok(Self(uuid))
    }

    pub fn as_string(&self) -> String {
        self.0.to_string()
    }
}

impl From<WalletID> for String {
    fn from(id: WalletID) -> Self {
        id.0.to_string()
    }
}

impl TryFrom<String> for WalletID {
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
            "invalid wallet id format, should be XXXXXXXX-XXXX-XXXX-XXXX-XXXXXXXXXXXX"
        );
        let id = Uuid::parse_str(&src).map_err(|_| anyhow!("invalid wallet id"))?;
        Ok(WalletID(id))
    }
}

impl From<WalletID> for Vec<u8> {
    fn from(id: WalletID) -> Self {
        id.0.as_bytes().to_vec()
    }
}

impl TryFrom<Vec<u8>> for WalletID {
    type Error = anyhow::Error;

    fn try_from(id: Vec<u8>) -> Result<Self> {
        let id = Uuid::from_slice(&id).map_err(|_| anyhow!("invalid wallet id"))?;
        Ok(Self(id))
    }
}
