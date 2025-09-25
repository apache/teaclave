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
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use types::external::{ClientExternalAddress, Email};

#[derive(Debug, Serialize, Clone, PartialEq, Eq, Hash)]
pub struct AddressName(pub String);

impl From<AddressName> for String {
    fn from(name: AddressName) -> String {
        name.0
    }
}

impl<'de> serde::de::Deserialize<'de> for AddressName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        let value_str: &str = Deserialize::deserialize(deserializer)?;
        if value_str.len() > 20 {
            return Err(serde::de::Error::custom("name too long, max 20 characters"));
        }
        if value_str.is_empty() {
            return Err(serde::de::Error::custom("name cannot be empty"));
        }
        Ok(AddressName(value_str.to_string()))
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AddressBookEntry {
    pub address: ClientExternalAddress,
    pub name: AddressName,
    pub last_modified_time: u64,
    pub last_modified_by: Email,
}

impl AddressBookEntry {
    pub fn new(
        address: ClientExternalAddress,
        name: AddressName,
        last_modified_by: Email,
    ) -> AddressBookEntry {
        AddressBookEntry {
            address,
            name,
            last_modified_time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            last_modified_by,
        }
    }

    pub fn new_null(address: ClientExternalAddress) -> AddressBookEntry {
        AddressBookEntry {
            address,
            name: AddressName("unknown".to_string()),
            last_modified_time: 0,
            last_modified_by: Email("admin@ck.com".to_string()),
        }
    }

    pub fn update_name(&mut self, name: AddressName, last_modified_by: Email) {
        self.name = name;
        self.last_modified_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_modified_by = last_modified_by;
    }
}

impl Storable<String> for AddressBookEntry {
    fn unique_id(&self) -> String {
        self.address.as_str().to_owned().to_ascii_lowercase()
    }
}
