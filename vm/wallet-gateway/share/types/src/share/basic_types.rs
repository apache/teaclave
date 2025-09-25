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

use crate::share::{CkHash, CkHasher};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

// key types
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CkPublicKey(#[serde(with = "hex::serde")] pub Vec<u8>);

impl CkPublicKey {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<Vec<u8>> for CkPublicKey {
    #[inline]
    fn from(value: Vec<u8>) -> CkPublicKey {
        CkPublicKey(value)
    }
}

impl From<CkPublicKey> for Vec<u8> {
    #[inline]
    fn from(value: CkPublicKey) -> Vec<u8> {
        value.0
    }
}

impl CkHasher for CkPublicKey {
    fn hash(&self) -> Result<CkHash> {
        CkHash::new(self.0.clone())
    }
}

#[derive(Serialize, Deserialize, Hash, Debug)]
pub struct CkPrivateKey(#[serde(with = "hex::serde")] Vec<u8>);

impl CkPrivateKey {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self(bytes.into())
    }
}

impl From<CkPrivateKey> for Vec<u8> {
    fn from(value: CkPrivateKey) -> Vec<u8> {
        value.0
    }
}

#[derive(Serialize, Deserialize, Hash, Debug, Clone)]
pub struct CkSignature(Vec<u8>);

impl CkSignature {
    pub fn new(signature: Vec<u8>) -> Self {
        Self(signature)
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl From<Vec<u8>> for CkSignature {
    fn from(value: Vec<u8>) -> CkSignature {
        CkSignature(value)
    }
}

impl From<CkSignature> for String {
    fn from(value: CkSignature) -> String {
        hex::encode(value.0)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CkEncryptedPayload {
    pub encrypted_data: Vec<u8>, // encrypted data by random data_key (AES-GCM-128)
    pub encrypted_data_key: Vec<u8>, // encrypted data_key by device_pubkey (RSA)
}
