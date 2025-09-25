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

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::net;
use std::path::Path;
use std::path::PathBuf;
use types::serde_util;
use types::share::NetworkType;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RuntimeConfig {
    pub internal_endpoints: InternalEndpointsConfig,
    pub blockchain_network: BlockchainNetworkConfig,
    pub task: TaskConfig,
    pub user_auth_info: UserAuthInfoConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InternalEndpointsConfig {
    pub api_server: InternalEndpoint,
    pub authority_server: InternalEndpoint,
    pub db_service: InternalEndpoint,
    pub tee_wallet: InternalEndpoint,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InternalEndpoint {
    pub listen_address: net::SocketAddr,
    pub advertised_address: url::Url,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockchainNetworkConfig {
    // network type
    pub network_type: NetworkType,
    // common
    pub currency_query_base_url: url::Url,
    #[serde(with = "serde_util::u128_string")]
    pub eth_default_gas_limit: u128,
    #[serde(with = "serde_util::u128_string")]
    pub erc20_default_gas_limit: u128,
    pub gas_price_percentage_increase: f64,
    pub delegation_attempt_intervals: u64,
    pub auto_sign: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskConfig {
    pub task_exec_intervals: u64,
    pub email_domain: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserAuthInfoConfig {
    pub auth_service_url: url::Url,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StorageConfig {
    pub certs_path: PathBuf,
    pub db_path: PathBuf,
    pub export_path: PathBuf,
    pub export_base_url: url::Url,
    pub log_path: PathBuf,
}

impl RuntimeConfig {
    pub fn from_toml<T: AsRef<Path>>(path: T) -> Result<Self> {
        let contents = fs::read_to_string(path.as_ref())
            .context("Something went wrong when reading the runtime config file")?;
        let config: RuntimeConfig =
            toml::from_str(&contents).context("Cannot parse the runtime config file")?;

        log::trace!(
            "Loaded config from {}: {:?}",
            path.as_ref().display(),
            config
        );
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_config() {
        let config = RuntimeConfig::from_toml("runtime_config_example.toml").unwrap();
        let addr = config.internal_endpoints.api_server.listen_address;
        println!("webapi: ip: {:?}, port: {:?}", addr.ip(), addr.port());

        let url = config.internal_endpoints.tee_wallet.advertised_address;
        let host = url.host().unwrap();
        let s = format!("{}", &host);
        println!(
            "tee_wallet: name: {:?}, port: {:?}, s= {:?}",
            url.host(),
            url.port(),
            s
        );
        println!(
            "server_name = {:?}",
            rustls::client::ServerName::try_from(s.as_str()).unwrap()
        );
    }
}
