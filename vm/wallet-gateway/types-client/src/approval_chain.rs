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
use types::external::{ApprovalStage, Email};
use types::share::ApprovalStatus;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalStatusInfo {
    email: Email,           // key
    status: ApprovalStatus, // value
    timestamp: Option<u64>, // value
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalStageInfo {
    threshold: u64,
    info: Vec<ApprovalStatusInfo>,
    status: ApprovalStatus,
}

impl From<ApprovalStage> for ApprovalStageInfo {
    fn from(stage: ApprovalStage) -> Self {
        let threshold = stage.get_threshold();
        let status = stage.get_stage_status();

        let mut info: Vec<ApprovalStatusInfo> = stage
            .take_status()
            .into_iter()
            .map(|(email, info)| ApprovalStatusInfo::new(email, info.status(), info.timestamp()))
            .collect();
        info.sort_by(|a, b| a.email.0.cmp(&b.email.0));
        Self {
            threshold,
            info,
            status,
        }
    }
}

impl ApprovalStageInfo {
    pub fn match_stage(&self, other: &ApprovalStage) -> bool {
        if self.threshold == other.get_threshold() {
            for approver_info in self.info.iter() {
                let other_approver_info = match other.get(&approver_info.email) {
                    Some(info) => info,
                    None => {
                        return false;
                    }
                };
                if approver_info.status != other_approver_info.status()
                    || approver_info.timestamp != other_approver_info.timestamp()
                {
                    return false;
                }
            }
            return true;
        }
        false
    }
}

impl ApprovalStatusInfo {
    pub fn new(email: Email, status: ApprovalStatus, timestamp: Option<u64>) -> Self {
        Self {
            email,
            status,
            timestamp,
        }
    }
}
