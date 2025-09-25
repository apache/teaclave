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
use types::share::{DeviceID, ServiceInfo, TeeOnlineStatus};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OnlineDevice {
    pub device_id: DeviceID,
    pub status: TeeOnlineStatus,
}

impl OnlineDevice {
    pub fn new(device_id: DeviceID, status: TeeOnlineStatus) -> Self {
        Self { device_id, status }
    }

    pub fn status(&self) -> &TeeOnlineStatus {
        &self.status
    }

    pub fn device_id(&self) -> DeviceID {
        self.device_id.clone()
    }

    pub fn set_running(&mut self, config_version: u64) {
        self.status = TeeOnlineStatus::ServiceRunning(ServiceInfo { config_version });
    }
}

impl Storable<String> for OnlineDevice {
    fn unique_id(&self) -> String {
        "OnlineDevice".to_string()
    }
}
