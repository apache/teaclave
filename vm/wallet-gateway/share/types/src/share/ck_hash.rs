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

use anyhow::Result;
use basic_utils::keccak_hash_to_bytes;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

pub const CK_HASH_SIZE: usize = 8;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash, Default)]
pub struct CkHash(#[serde(with = "hex::serde")] pub [u8; CK_HASH_SIZE]);

impl CkHash {
    pub fn new(data: Vec<u8>) -> Result<Self> {
        Ok(Self(
            keccak_hash_to_bytes(&data)[..CK_HASH_SIZE].try_into()?,
        ))
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }
}

pub trait CkHasher {
    fn hash(&self) -> Result<CkHash>;
}
