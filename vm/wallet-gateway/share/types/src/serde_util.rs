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

use serde::{Deserialize, Deserializer, Serializer};

pub mod u128_string {
    use super::*;

    pub fn serialize<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value_str: &str = Deserialize::deserialize(deserializer)?;
        value_str.parse().map_err(serde::de::Error::custom)
    }
}

pub mod u128_hex {
    use super::*;

    pub fn serialize<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{:x}", value))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value_str: &str = Deserialize::deserialize(deserializer)?;
        u128::from_str_radix(value_str.strip_prefix("0x").unwrap_or(value_str), 16)
            .map_err(serde::de::Error::custom)
    }
}

pub mod option_u128_hex {
    use super::*;

    pub fn serialize<S>(value: &Option<u128>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(value) => serializer.serialize_some(&format!("0x{:x}", value)),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u128>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value_str: Option<&str> = Deserialize::deserialize(deserializer)?;
        match value_str {
            Some(value_str) => {
                u128::from_str_radix(value_str.strip_prefix("0x").unwrap_or(value_str), 16)
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            }
            None => Ok(None),
        }
    }
}

pub mod bytes_hex {
    use super::*;

    pub fn serialize<S>(value: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("0x{}", hex::encode(value)))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value_str: &str = Deserialize::deserialize(deserializer)?;
        hex::decode(value_str.strip_prefix("0x").unwrap_or(value_str))
            .map_err(serde::de::Error::custom)
    }
}

pub mod f64_string {
    use super::*;

    pub fn serialize<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<f64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value_str: &str = Deserialize::deserialize(deserializer)?;
        value_str.parse().map_err(serde::de::Error::custom)
    }
}

pub mod url_string_for_email {
    use super::*;

    pub fn serialize<S>(value: &url::Url, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // serialize as href="url"
        serializer.serialize_str(&format!("href=\"{}\"", value))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<url::Url, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value_str: &str = Deserialize::deserialize(deserializer)?;
        url::Url::parse(value_str).map_err(serde::de::Error::custom)
    }
}
