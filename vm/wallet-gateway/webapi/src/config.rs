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
use ck_config::{BlockchainNetworkConfig, RuntimeConfig};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

#[derive(Debug)]
pub struct WebapiConfig {
    http_server_config: HttpServerConfig,
    shared_state_config: Arc<SharedStateConfig>,
    delegation_config: DelegationConfig,
}
impl WebapiConfig {
    pub fn new(root_path: &Path, config_file_path: &Path) -> Result<Self> {
        let runtime_config = RuntimeConfig::from_toml(root_path.join(config_file_path))?;
        let infura_token = std::env::var("INFURA_TOKEN").expect("INFURA_TOKEN not set");

        let shared_state_config = SharedStateConfig {
            tee_server_url: runtime_config
                .internal_endpoints
                .tee_wallet
                .advertised_address,
            network_config: runtime_config.blockchain_network.clone(),
            db_path: root_path.join(runtime_config.storage.db_path),
            db_server_url: runtime_config
                .internal_endpoints
                .db_service
                .advertised_address,
            certs_path: root_path.join(runtime_config.storage.certs_path),
            auth_service_url: runtime_config.user_auth_info.auth_service_url,
            auto_sign: runtime_config.blockchain_network.auto_sign,
            infura_token,
            log_path: root_path.join(runtime_config.storage.log_path),
        };

        Ok(Self {
            http_server_config: HttpServerConfig::new(
                runtime_config.internal_endpoints.api_server.listen_address,
            )?,
            shared_state_config: Arc::new(shared_state_config),
            delegation_config: DelegationConfig::new(
                runtime_config
                    .blockchain_network
                    .delegation_attempt_intervals,
            )?,
        })
    }
    pub fn http_server_config(&self) -> &HttpServerConfig {
        &self.http_server_config
    }
    pub fn shared_state_config(&self) -> Arc<SharedStateConfig> {
        self.shared_state_config.clone()
    }
    pub fn delegation_config(&self) -> &DelegationConfig {
        &self.delegation_config
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HttpServerConfig {
    listen_address: SocketAddr,
    ip: IpAddr,
    port: u16,
}
impl HttpServerConfig {
    pub fn new(addr: SocketAddr) -> Result<Self> {
        let ip = addr.ip();
        let port = addr.port();
        Ok(Self {
            listen_address: addr,
            ip,
            port,
        })
    }
    pub fn ip(&self) -> IpAddr {
        self.ip
    }
    pub fn port(&self) -> u16 {
        self.port
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DelegationConfig {
    pub delegation_attempt_intervals: u64,
}
impl DelegationConfig {
    pub fn new(delegation_attempt_intervals: u64) -> Result<Self> {
        Ok(Self {
            delegation_attempt_intervals,
        })
    }
}

#[derive(Debug)]
pub struct SharedStateConfig {
    pub tee_server_url: url::Url,
    pub db_path: PathBuf,
    pub db_server_url: url::Url,
    pub certs_path: PathBuf,
    pub network_config: BlockchainNetworkConfig,
    pub auth_service_url: url::Url,
    pub auto_sign: bool,
    pub infura_token: String,
    pub log_path: PathBuf,
}
