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

use crate::share::UserID;
use anyhow::{ensure, Result};
use basic_utils::keccak_hash_to_bytes;
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

// user email
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Email(pub String);

impl TryFrom<String> for Email {
    type Error = anyhow::Error;

    fn try_from(src: String) -> Result<Self> {
        // check email format
        let parts: Vec<&str> = src.split('@').collect();
        ensure!(
            parts.len() == 2 && !parts[0].is_empty() && parts[1].contains('.'),
            "invalid email format"
        );
        Ok(Email(src))
    }
}

impl From<Email> for String {
    fn from(email: Email) -> Self {
        email.0
    }
}

impl From<Email> for UserID {
    fn from(email: Email) -> Self {
        let mut bytes = [0u8; 8];
        let v = keccak_hash_to_bytes(&email.0);
        bytes.copy_from_slice(&v[..8]);
        UserID::new(bytes)
    }
}
