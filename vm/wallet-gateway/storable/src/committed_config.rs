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
use serde::{Deserialize, Serialize};
use types::share::{DeviceID, TeeConfig};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CommittedConfig {
    pub device_id: DeviceID,
    pub timestamp: u64,
    pub signed_config: TeeConfig,
}

impl Storable<DeviceID> for CommittedConfig {
    fn unique_id(&self) -> DeviceID {
        self.device_id.clone()
    }
}

impl CommittedConfig {
    pub fn new(device_id: DeviceID, timestamp: u64, signed_config: TeeConfig) -> Self {
        Self {
            device_id,
            timestamp,
            signed_config,
        }
    }

    pub fn config_version(&self) -> u64 {
        self.signed_config.config_version
    }
}
