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

use crate::{DBCompatibleClient, DBCredentials};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// unused, reserved for AWS tokens
pub struct LocalServiceCredentials {}
impl DBCredentials for LocalServiceCredentials {}

pub struct LocalServiceClient {
    address: String, // ip:port, http://localhost:8543
    req_client: reqwest::Client,
}
#[async_trait]
impl DBCompatibleClient for LocalServiceClient {
    type T = LocalServiceCredentials;
    async fn init(db_name: &str, _credentials: Option<Self::T>) -> Result<Self> {
        Ok(Self {
            address: db_name.to_string(),
            req_client: reqwest::Client::new(),
        })
    }

    async fn get_value(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let request = GetRequest {
            key: key.as_bytes().to_vec(),
        };
        let response = self
            .req_client
            .post(format!("{}get", self.address))
            .json(&request)
            .send()
            .await?
            .json::<GetResponse>()
            .await?;
        Ok(response.value)
    }

    async fn put_value(&self, key: &str, value: &[u8]) -> Result<()> {
        let request = PutRequest {
            key: key.as_bytes().to_vec(),
            value: value.to_vec(),
        };
        match self
            .req_client
            .post(format!("{}put", self.address))
            .json(&request)
            .send()
            .await?
            .json::<PutResponse>()
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                log::error!("put_value(): error: {:?}", e);
                Ok(())
            }
        }
    }

    async fn delete_entry(&self, key: &str) -> Result<()> {
        let request = DeleteRequest {
            key: key.as_bytes().to_vec(),
        };
        match self
            .req_client
            .post(format!("{}delete", self.address))
            .json(&request)
            .send()
            .await?
            .json::<DeleteResponse>()
            .await
        {
            Ok(_response) => Ok(()),
            Err(e) => {
                log::error!("delete_entry(): error: {:?}", e);
                Ok(())
            }
        }
    }

    async fn list_entries(&self) -> Result<HashMap<String, Vec<u8>>> {
        let request = ListRequest {};
        let response = self
            .req_client
            .post(format!("{}list", self.address))
            .json(&request)
            .send()
            .await?
            .json::<ListResponse>()
            .await?;
        Ok(response.map)
    }

    async fn list_entries_with_prefix(&self, prefix: &str) -> Result<HashMap<String, Vec<u8>>> {
        let request = ListByPrefixRequest {
            prefix: prefix.to_string(),
        };
        let response = self
            .req_client
            .post(format!("{}list_by_prefix", self.address))
            .json(&request)
            .send()
            .await?
            .json::<ListByPrefixResponse>()
            .await?;
        Ok(response.map)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetRequest {
    pub key: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetResponse {
    pub value: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PutRequest {
    pub key: Vec<u8>,
    pub value: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PutResponse {}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub key: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteResponse {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListRequest {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse {
    pub map: HashMap<String, Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListByPrefixRequest {
    pub prefix: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListByPrefixResponse {
    pub map: HashMap<String, Vec<u8>>, // key -> value
}
