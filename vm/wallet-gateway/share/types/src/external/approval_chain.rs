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
use crate::external::{ApprovalChainBasic, ApprovalStageBasic};
use crate::share::ApprovalStatus;
use crate::share::UserID;
use crate::share::{TaApprovalChain, TaApprovalStage};
use anyhow::{anyhow, ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::iter::IntoIterator;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalInfo {
    status: ApprovalStatus,
    timestamp: Option<u64>,
}

impl ApprovalInfo {
    pub fn new(status: ApprovalStatus) -> Self {
        Self {
            status,
            timestamp: None,
        }
    }

    pub fn status(&self) -> ApprovalStatus {
        self.status
    }

    pub fn timestamp(&self) -> Option<u64> {
        self.timestamp
    }

    pub fn try_update_status(&mut self, upcoming_status: ApprovalStatus) -> Result<()> {
        if self.status == upcoming_status {
            return Ok(());
        }
        self.status = upcoming_status;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        self.timestamp = Some(now);
        Ok(())
    }

    pub fn is_operated(&self) -> bool {
        self.status != ApprovalStatus::Pending
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalStage {
    threshold: u64,
    status: HashMap<Email, ApprovalInfo>,
}

impl ApprovalStage {
    pub fn new(threshold: u64, approvers: HashSet<Email>) -> Self {
        Self {
            threshold,
            status: approvers
                .into_iter()
                .map(|a| (a, ApprovalInfo::new(ApprovalStatus::Pending)))
                .collect(),
        }
    }

    pub fn try_update(
        &mut self,
        ta_stage: TaApprovalStage,
        uid_to_email: &HashMap<UserID, Email>,
    ) -> Result<()> {
        ensure!(
            self.threshold == ta_stage.get_threshold(),
            "ApprovalStage::try_update: threshold not match"
        );

        let mut converted_stage = HashMap::new();
        for (uid, upcoming_status) in ta_stage.status.into_iter() {
            let email = uid_to_email
                .get(&uid)
                .ok_or_else(|| anyhow!("ApprovalStage::try_update: uid not found"))?
                .clone();
            converted_stage.insert(email, upcoming_status);
        }

        ensure!(
            self.status.len() == converted_stage.len()
                && self.status.keys().all(|k| converted_stage.contains_key(k)),
            "ApprovalStage::try_update: email not match"
        );

        // safety: unwrap never panic, as we have already checked the keys
        self.status.iter_mut().try_for_each(|(email, info)| {
            info.try_update_status(*converted_stage.get(email).unwrap())
        })?;
        Ok(())
    }

    pub fn uid_email_mapping(&self) -> HashMap<UserID, Email> {
        self.status
            .keys()
            .map(|email| (UserID::from(email.clone()), email.clone()))
            .collect()
    }

    pub fn get_threshold(&self) -> u64 {
        self.threshold
    }

    pub fn take_status(self) -> HashMap<Email, ApprovalInfo> {
        self.status
    }

    pub fn get_status(&self) -> HashMap<Email, ApprovalInfo> {
        self.status.clone()
    }

    pub fn status(&self) -> &HashMap<Email, ApprovalInfo> {
        &self.status
    }

    pub fn is_approved(&self) -> bool {
        self.status
            .values()
            .filter(|s| s.status() == ApprovalStatus::Approved)
            .count() as u64
            >= self.threshold
    }

    pub fn is_rejected(&self) -> bool {
        self.status
            .values()
            .any(|s| s.status() == ApprovalStatus::Rejected)
    }

    pub fn contains(&self, user: &Email) -> bool {
        self.status.keys().any(|k| k == user)
    }

    pub fn get_stage_status(&self) -> ApprovalStatus {
        if self.is_rejected() {
            ApprovalStatus::Rejected
        } else if self.is_approved() {
            ApprovalStatus::Approved
        } else {
            ApprovalStatus::Pending
        }
    }
}

impl std::ops::Deref for ApprovalStage {
    type Target = HashMap<Email, ApprovalInfo>;

    fn deref(&self) -> &Self::Target {
        &self.status
    }
}

impl From<ApprovalStageBasic> for ApprovalStage {
    fn from(stage: ApprovalStageBasic) -> Self {
        Self {
            threshold: stage.threshold,
            status: stage
                .approvers
                .into_iter()
                .map(|a| (a, ApprovalInfo::new(ApprovalStatus::Pending)))
                .collect(),
        }
    }
}

impl From<ApprovalStage> for TaApprovalStage {
    fn from(asb: ApprovalStage) -> Self {
        let threshold = asb.get_threshold();
        let status = asb
            .get_status()
            .into_iter()
            .map(|(k, v)| (k.into(), v.status()))
            .collect();
        TaApprovalStage::from((threshold, status))
    }
}

impl From<ApprovalChain> for TaApprovalChain {
    fn from(ac: ApprovalChain) -> Self {
        TaApprovalChain::new(ac.into_iter().map(|e| e.into()).collect::<Vec<_>>())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalChain(Vec<ApprovalStage>);

impl ApprovalChain {
    pub fn new(inner: Vec<ApprovalStage>) -> Self {
        Self(inner)
    }

    pub fn try_update(
        &mut self,
        ta_chain: TaApprovalChain,
        uid_to_email: &HashMap<UserID, Email>,
    ) -> Result<()> {
        ensure!(
            self.0.len() == ta_chain.len(),
            "ApprovalChain::try_update: stage not match"
        );

        let ta_stages = ta_chain.take_stages();
        for (i, ta_stage) in ta_stages.into_iter().enumerate() {
            let entry = self
                .0
                .get_mut(i)
                .ok_or_else(|| anyhow!("ApprovalChain::try_update: stage not match"))?;
            entry.try_update(ta_stage, uid_to_email)?;
        }
        Ok(())
    }

    pub fn any_reject(&self) -> bool {
        self.0
            .iter()
            .any(|s| s.get_stage_status() == ApprovalStatus::Rejected)
    }

    pub fn any_pending(&self) -> bool {
        self.0
            .iter()
            .any(|s| s.get_stage_status() == ApprovalStatus::Pending)
    }

    pub fn all_approved(&self) -> bool {
        self.0
            .iter()
            .all(|s| s.get_stage_status() == ApprovalStatus::Approved)
    }

    pub fn current_stage_index(&self) -> Option<usize> {
        self.0
            .iter()
            .position(|s| s.get_stage_status() == ApprovalStatus::Pending)
    }

    pub fn ready_for_current_stage(&self, user: &Email) -> bool {
        if let Some(stage) = self.current_stage_index() {
            self.0[stage].contains(user)
        } else {
            false
        }
    }

    pub fn approvers_on_stage(&self, stage: usize) -> HashSet<Email> {
        self.0[stage].status.keys().cloned().collect()
    }

    // operated approvers are those who have approved or rejected
    pub fn get_operated_approvers(&self) -> HashSet<Email> {
        let mut operated_approvers = HashSet::new();
        for stage in self.0.iter() {
            for (approver, info) in stage.status.iter() {
                if info.is_operated() {
                    operated_approvers.insert(approver.clone());
                }
            }
        }
        operated_approvers
    }

    pub fn approvers(&self) -> HashSet<Email> {
        self.0
            .iter()
            .flat_map(|s| s.status.keys().cloned())
            .collect()
    }

    pub fn uid_email_mapping(&self) -> HashMap<UserID, Email> {
        self.0
            .iter()
            .flat_map(|s| s.uid_email_mapping().into_iter())
            .collect()
    }
}

impl std::ops::Deref for ApprovalChain {
    type Target = Vec<ApprovalStage>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ApprovalChain {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<ApprovalChainBasic> for ApprovalChain {
    fn from(chain: ApprovalChainBasic) -> Self {
        Self(chain.into_iter().map(|stage| stage.into()).collect())
    }
}

impl IntoIterator for ApprovalChain {
    type Item = ApprovalStage;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
