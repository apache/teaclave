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

use ck_config::RuntimeConfig;
use std::{
    net::IpAddr,
    path::{Path, PathBuf},
};
use url::Url;

pub struct AuthorityConfig {
    pub cert_path: PathBuf,        // certs: ca.cert, ca.der, ca.key
    pub export_base_path: PathBuf, // export: device.cert, signed files
    pub export_base_url: Url,      // exported files for download
    pub log_path: PathBuf,         // log: log file
    pub auth_service_url: Url,
    pub db_server_url: Url,
    pub ip: IpAddr,
    pub port: u16,
}
impl AuthorityConfig {
    pub fn new(root_path: &Path, config_file_path: &Path) -> Self {
        let runtime_config = RuntimeConfig::from_toml(root_path.join(config_file_path))
            .expect("failed to read runtime config");

        Self {
            cert_path: root_path.join(runtime_config.storage.certs_path),
            export_base_path: root_path.join(runtime_config.storage.export_path),
            export_base_url: runtime_config.storage.export_base_url,
            log_path: root_path.join(runtime_config.storage.log_path),
            auth_service_url: runtime_config.user_auth_info.auth_service_url,
            db_server_url: runtime_config
                .internal_endpoints
                .db_service
                .advertised_address,
            ip: runtime_config
                .internal_endpoints
                .authority_server
                .listen_address
                .ip(),
            port: runtime_config
                .internal_endpoints
                .authority_server
                .listen_address
                .port(),
        }
    }
}
