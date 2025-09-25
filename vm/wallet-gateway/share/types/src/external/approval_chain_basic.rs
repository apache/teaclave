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

use crate::external::Email;
use crate::share::{TaApprovalChainBasic, TaApprovalStageBasic};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::iter::IntoIterator;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalStageBasic {
    pub threshold: u64,
    pub approvers: HashSet<Email>,
}

impl ApprovalStageBasic {
    pub fn new(threshold: u64, approvers: HashSet<Email>) -> Self {
        Self {
            threshold,
            approvers,
        }
    }
}

impl From<ApprovalStageBasic> for TaApprovalStageBasic {
    fn from(asb: ApprovalStageBasic) -> Self {
        Self {
            threshold: asb.threshold,
            approvers: asb.approvers.into_iter().map(|e| e.into()).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalChainBasic(Vec<ApprovalStageBasic>);

impl ApprovalChainBasic {
    pub fn new(inner: Vec<ApprovalStageBasic>) -> Self {
        Self(inner)
    }

    pub fn distinct_approvers(&self) -> HashSet<&Email> {
        self.0
            .iter()
            .flat_map(|stage| stage.approvers.iter())
            .collect()
    }

    pub fn contains(&self, user: &Email) -> bool {
        self.0.iter().any(|stage| stage.approvers.contains(user))
    }
}

impl IntoIterator for ApprovalChainBasic {
    type Item = ApprovalStageBasic;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl std::iter::FromIterator<ApprovalStageBasic> for ApprovalChainBasic {
    fn from_iter<I: IntoIterator<Item = ApprovalStageBasic>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl std::ops::Deref for ApprovalChainBasic {
    type Target = Vec<ApprovalStageBasic>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ApprovalChainBasic> for TaApprovalChainBasic {
    fn from(acb: ApprovalChainBasic) -> Self {
        Self::new(acb.into_iter().map(|e| e.into()).collect::<Vec<_>>())
    }
}
