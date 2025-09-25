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
use async_trait::async_trait;
#[cfg(feature = "rocks-db")]
use rocksdb::{DBWithThreadMode, IteratorMode, MultiThreaded};
use std::collections::HashMap;

pub trait DBCredentials {}

#[async_trait]
pub trait DBCompatibleClient: Send + Sync {
    type T: DBCredentials;
    async fn init(db_name: &str, credentials: Option<Self::T>) -> Result<Self>
    where
        Self: Sized;
    async fn get_value(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn put_value(&self, key: &str, value: &[u8]) -> Result<()>;
    async fn delete_entry(&self, key: &str) -> Result<()>;
    async fn list_entries(&self) -> Result<HashMap<String, Vec<u8>>>;
    async fn list_entries_with_prefix(&self, prefix: &str) -> Result<HashMap<String, Vec<u8>>>;
}

#[cfg(feature = "rocks-db")]
pub struct RocksDBCredentials {}
#[cfg(feature = "rocks-db")]
impl DBCredentials for RocksDBCredentials {}
#[cfg(feature = "rocks-db")]
pub struct RocksDBClient {
    db: DBWithThreadMode<MultiThreaded>,
}
#[async_trait]
#[cfg(feature = "rocks-db")]
impl DBCompatibleClient for RocksDBClient {
    type T = RocksDBCredentials;
    async fn init(db_name: &str, _credentials: Option<Self::T>) -> Result<Self> {
        let db = DBWithThreadMode::<MultiThreaded>::open_default(db_name)?;
        Ok(Self { db })
    }

    async fn get_value(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self.db.get(key) {
            Ok(Some(value)) => Ok(Some(value)),
            Ok(None) => Ok(None),
            Err(e) => {
                bail!("rocksdb get error: {}", e);
            }
        }
    }

    async fn put_value(&self, key: &str, value: &[u8]) -> Result<()> {
        self.db.put(key, value)?;
        Ok(())
    }

    async fn delete_entry(&self, key: &str) -> Result<()> {
        self.db.delete(key)?;
        Ok(())
    }

    async fn list_entries(&self) -> Result<HashMap<String, Vec<u8>>> {
        let mut map = HashMap::new();
        let iter = self.db.iterator(IteratorMode::Start);
        for item in iter {
            let (key, value) = item?;
            map.insert(String::from_utf8(key.to_vec())?, value.to_vec());
        }
        Ok(map)
    }

    async fn list_entries_with_prefix(&self, prefix: &str) -> Result<HashMap<String, Vec<u8>>> {
        let mut map: HashMap<String, Vec<u8>> = HashMap::new();
        let iter = self.db.iterator(IteratorMode::Start);
        for item in iter {
            let (key, value) = item?;

            let key = String::from_utf8(key.to_vec())?;
            if key.starts_with(prefix) {
                map.insert(key, value.to_vec());
            }
        }
        Ok(map)
    }
}
