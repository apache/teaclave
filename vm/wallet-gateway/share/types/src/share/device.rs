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

use crate::share::CkPublicKey;
use anyhow::{ensure, Result};
use basic_utils::keccak_hash_to_bytes;
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::hash::Hash;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct DeviceID(#[serde(with = "hex::serde")] [u8; 8]); // <hash of public key>

impl DeviceID {
    pub fn new(id: impl Into<Vec<u8>>) -> Result<Self> {
        let id: Vec<u8> = id.into();
        ensure!(
            id.len() == 8,
            "invalid device id format, should be 8 bytes of hash"
        );
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&id[..8]);
        Ok(Self(bytes))
    }
}

impl From<CkPublicKey> for DeviceID {
    fn from(public_key: CkPublicKey) -> Self {
        Self(
            keccak_hash_to_bytes(&public_key.0)[..8]
                .try_into()
                .unwrap_or_default(),
        )
    }
}

impl From<DeviceID> for String {
    fn from(device_id: DeviceID) -> Self {
        hex::encode(device_id.0)
    }
}

impl TryFrom<String> for DeviceID {
    type Error = anyhow::Error;

    fn try_from(src: String) -> std::result::Result<Self, Self::Error> {
        // check device id format
        ensure!(
            src.len() == 16,
            "invalid device id format, should be hex string of 16 length"
        );
        DeviceID::new(hex::decode(src)?)
    }
}
