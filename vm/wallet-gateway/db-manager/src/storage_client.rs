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

// abstraction layer for webapi & authority

use crate::{db_compatible_client::DBCompatibleClient, LocalServiceCredentials};
use anyhow::Result;
use core::hash::Hash;
use std::collections::HashMap;
use std::sync::Arc;
use storable::Storable;
use tokio::sync::RwLock;

pub struct StorageClient {
    client: Box<dyn DBCompatibleClient<T = LocalServiceCredentials>>,
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}
impl StorageClient {
    pub fn new(client: Box<dyn DBCompatibleClient<T = LocalServiceCredentials>>) -> Self {
        Self {
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    pub async fn get<K, V>(&self, key: &K) -> Result<Option<V>>
    where
        K: TryFrom<String> + Into<String> + Clone,
        V: Storable<K>,
    {
        let key: String = (*key).clone().into();
        let storage_key = V::concat_key(&key);
        log::debug!("get(): storage_key: {}", storage_key);

        match self.client.get_value(&storage_key).await? {
            Some(value) => Ok(Some(serde_json::from_slice(&value)?)),
            None => Ok(None),
        }
    }
    pub async fn put<K, V>(&self, value: &V) -> Result<()>
    where
        K: TryFrom<String> + Into<String> + Clone,
        V: Storable<K>,
    {
        let key = value.storage_key();
        log::debug!("put(): storage_key: {}", key);
        let value = serde_json::to_vec(value)?;
        self.client.put_value(&key, &value).await?;

        Ok(())
    }
    pub async fn delete_entry<K, V>(&self, key: &K) -> Result<()>
    where
        K: TryFrom<String> + Into<String> + Clone,
        V: Storable<K>,
    {
        let key: String = (*key).clone().into();
        let storage_key = V::concat_key(&key);
        self.client.delete_entry(&storage_key).await?;

        Ok(())
    }
    pub async fn list_entries<K, V>(&self) -> Result<HashMap<K, V>>
    where
        K: TryFrom<String> + Into<String> + Clone + Eq + Hash,
        V: Storable<K>,
    {
        let mut entries = HashMap::new();
        log::debug!("list_entries: table_name: {}", V::table_name());
        let map = self
            .client
            .list_entries_with_prefix(V::table_name())
            .await?;
        for (key, value) in map {
            let value: V = serde_json::from_slice(&value)?;
            entries.insert(value.unique_id().to_owned(), value);
        }
        Ok(entries)
    }
}
