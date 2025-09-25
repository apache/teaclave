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

use crate::share::UserID;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// note: keep the name consistent with the one in external:
// [Ta]ApprovalChainBasic: only contains user identity, such as UserID, Email, used for wallet initialization
// [Ta]ApprovalChain: contains user identity and approval status, used for transaction approval

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaApprovalStageBasic {
    pub threshold: u64, // 1 <= threshold <= approvers.len()
    pub approvers: HashSet<UserID>,
}

impl TaApprovalStageBasic {
    pub fn new(threshold: u64, approvers: HashSet<UserID>) -> Self {
        Self {
            threshold,
            approvers,
        }
    }

    pub fn approvers(&self) -> &HashSet<UserID> {
        &self.approvers
    }

    pub fn threshold(&self) -> u64 {
        self.threshold
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaApprovalChainBasic(Vec<TaApprovalStageBasic>);

impl TaApprovalChainBasic {
    pub fn new(inner: Vec<TaApprovalStageBasic>) -> Self {
        Self(inner)
    }

    pub fn stages(&self) -> &Vec<TaApprovalStageBasic> {
        &self.0
    }

    pub fn distinct_approvers(&self) -> HashSet<&UserID> {
        self.0
            .iter()
            .flat_map(|s| s.approvers.iter())
            .collect::<HashSet<_>>()
    }
}

impl IntoIterator for TaApprovalChainBasic {
    type Item = TaApprovalStageBasic;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
