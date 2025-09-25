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

use serde::{Deserialize, Serialize};
use types::share::{ServiceInfo, TeeOnlineStatus};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "teeStatus")]
pub enum TeeStatusForClient {
    ServiceRunning(ServiceInfo),
    WaitingForSync,
    ServicePaused,
    Offline, // If tee cannot be reached, return this status to user
}

impl Default for TeeStatusForClient {
    fn default() -> Self {
        Self::Offline
    }
}

impl From<TeeOnlineStatus> for TeeStatusForClient {
    fn from(tee_online_status: TeeOnlineStatus) -> Self {
        match tee_online_status {
            TeeOnlineStatus::ServiceRunning(service_info) => Self::ServiceRunning(service_info),
            TeeOnlineStatus::WaitingForSync => Self::WaitingForSync,
            TeeOnlineStatus::ServicePaused => Self::ServicePaused,
        }
    }
}
