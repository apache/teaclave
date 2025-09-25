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
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TeeOnlineStatus {
    ServiceRunning(ServiceInfo),
    WaitingForSync,
    ServicePaused,
}

impl TeeOnlineStatus {
    pub fn init() -> Self {
        Self::WaitingForSync
    }

    pub fn set_running(&mut self, config_version: u64) {
        let tee_online_info = ServiceInfo { config_version };
        *self = Self::ServiceRunning(tee_online_info);
    }

    pub fn update_config_version(&mut self, config_version: u64) {
        if let Self::ServiceRunning(tee_online_info) = self {
            tee_online_info.config_version = config_version;
        }
    }

    pub fn config_version(&self) -> Result<u64> {
        match self {
            Self::ServiceRunning(tee_online_info) => Ok(tee_online_info.config_version),
            _ => Err(anyhow!("TEE service is not running")),
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self, Self::ServiceRunning(_))
    }

    pub fn is_waiting_for_sync(&self) -> bool {
        matches!(self, Self::WaitingForSync)
    }
}

impl Storable<String> for TeeOnlineStatus {
    fn unique_id(&self) -> String {
        "tee_online_status".to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceInfo {
    pub config_version: u64,
}
