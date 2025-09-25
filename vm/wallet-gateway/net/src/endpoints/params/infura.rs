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

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct InfuraJsonRpcSuccessResponse {
    jsonrpc: String,
    id: u32,
    result: String,
}
impl InfuraJsonRpcSuccessResponse {
    pub fn result(&self) -> String {
        self.result.clone()
    }
}
#[derive(Debug, Serialize, Deserialize)]
pub struct InfuraJsonRpcErrorResponse {
    jsonrpc: String,
    id: u32,
    error: RpcErrorInfo,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct RpcErrorInfo {
    code: i32,
    message: String,
}

pub type HttpStatusCode = u16;
#[derive(Error, Debug)]
pub enum InfuraRpcError {
    #[error("Infura connection error: {0}")]
    ConnectionFailure(HttpStatusCode), //when request.status() != 200
    #[error("Rejected by network: {0}")]
    RejectedByNetwork(String), // when deserialization to InfuraJsonRpcErrorResponse succeeds
    #[error("Unknown error: {0}")]
    Unknown(String), // when deserialization fails
}

pub async fn parse_infura_json_rpc_response(
    resp: reqwest::Response,
) -> Result<InfuraJsonRpcSuccessResponse> {
    if resp.status() != 200 {
        log::error!("Infura connection error code: {}", resp.status());
        return Err(InfuraRpcError::ConnectionFailure(resp.status().as_u16()).into());
    }
    let resp_text = resp.text().await?;
    log::info!("Infura response body: {}", resp_text);
    match serde_json::from_str::<InfuraJsonRpcSuccessResponse>(&resp_text) {
        Ok(resp) => Ok(resp),
        Err(_) => match serde_json::from_str::<InfuraJsonRpcErrorResponse>(&resp_text) {
            Ok(err_resp) => bail!(InfuraRpcError::RejectedByNetwork(err_resp.error.message)),
            Err(_) => bail!(InfuraRpcError::Unknown(resp_text)),
        },
    }
}

pub trait RpcParams: Serialize + Clone {}
impl RpcParams for String {}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InfuraRpcRequest<T>
where
    T: RpcParams,
{
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<T>,
    pub id: u32,
}
impl<T> InfuraRpcRequest<T>
where
    T: RpcParams,
{
    pub fn new(method: String, params: Vec<T>) -> Self {
        InfuraRpcRequest {
            jsonrpc: "2.0".to_string(),
            method,
            params,
            id: 1,
        }
    }
}
