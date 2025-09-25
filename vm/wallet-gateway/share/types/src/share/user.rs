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

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use std::hash::Hash;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct UserID(#[serde(with = "hex::serde")] [u8; 8]); // hash of public key

impl UserID {
    pub fn new(id: [u8; 8]) -> Self {
        Self(id)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl TryFrom<Vec<u8>> for UserID {
    type Error = anyhow::Error;

    fn try_from(id: Vec<u8>) -> Result<Self> {
        ensure!(
            id.len() == 8,
            "invalid user id format, should be 8 bytes of hash"
        );
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&id[..8]);
        Ok(Self(bytes))
    }
}

impl From<UserID> for String {
    fn from(user_id: UserID) -> Self {
        hex::encode(user_id.0)
    }
}

impl TryFrom<String> for UserID {
    type Error = anyhow::Error;

    fn try_from(src: String) -> std::result::Result<Self, Self::Error> {
        // check user id format
        ensure!(
            src.len() == 16,
            "invalid user id format, should be hex string of 16 length"
        );
        let v: Vec<u8> = src.into();
        UserID::try_from(v)
    }
}

impl Default for UserID {
    fn default() -> Self {
        Self([0xFFu8; 8])
    }
}

// user role
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Admin,
    Approver,
    TxOperator,
    System,
    Viewer, // unused for TA, just keep it for consistency
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct RoleSet(pub Vec<Role>);

impl RoleSet {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, role: Role) {
        self.0.push(role);
    }

    pub fn insert(&mut self, role: Role) {
        // if role exists, do nothing
        if self.0.contains(&role) {
            return;
        }
        self.push(role);
    }
}

impl Default for RoleSet {
    fn default() -> Self {
        Self::new()
    }
}

impl Iterator for RoleSet {
    type Item = Role;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.pop()
    }
}

impl std::ops::Deref for RoleSet {
    type Target = Vec<Role>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
