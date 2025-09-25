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
use std::collections::HashMap;
use types::external::Email;

#[derive(Serialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct NicknameKey(String);

impl From<String> for NicknameKey {
    fn from(s: String) -> Self {
        NicknameKey(s)
    }
}

#[derive(Serialize, Debug, Clone, Hash, Eq, PartialEq)]
pub struct NicknameValue(String);

impl From<String> for NicknameValue {
    fn from(s: String) -> Self {
        NicknameValue(s)
    }
}

impl NicknameValue {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> serde::de::Deserialize<'de> for NicknameKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let value_str: &str = Deserialize::deserialize(deserializer)?;
        // should accept bitcoin taproot address length
        if value_str.len() > 62 {
            return Err(serde::de::Error::custom("NicknameKey length exceeded"));
        }
        if value_str.is_empty() {
            return Err(serde::de::Error::custom("NicknameKey empty"));
        }
        Ok(NicknameKey(value_str.to_string()))
    }
}

impl<'de> serde::de::Deserialize<'de> for NicknameValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let value_str: &str = Deserialize::deserialize(deserializer)?;
        if value_str.len() > 20 {
            return Err(serde::de::Error::custom("NicknameValue length exceeded"));
        }
        if value_str.is_empty() {
            return Err(serde::de::Error::custom("NicknameValue empty"));
        }
        Ok(NicknameValue(value_str.to_string()))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NicknameInfo {
    email: Email,
    inner: HashMap<NicknameKey, NicknameValue>,
}

impl Storable<Email> for NicknameInfo {
    fn table_name() -> &'static str {
        "NicknameInfo"
    }
    fn unique_id(&self) -> Email {
        self.email.clone()
    }
}

impl NicknameInfo {
    pub fn new(email: Email) -> Self {
        Self {
            email,
            inner: HashMap::new(),
        }
    }

    pub fn take_all(self) -> HashMap<NicknameKey, NicknameValue> {
        self.inner
    }

    pub fn get_nickname(&self, k: &NicknameKey) -> Option<NicknameValue> {
        self.inner.get(k).cloned()
    }

    pub fn set_nickname(&mut self, k: NicknameKey, v: NicknameValue) {
        self.inner.insert(k, v);
    }

    pub fn remove_nickname(&mut self, k: &NicknameKey) {
        self.inner.remove(k);
    }
}
