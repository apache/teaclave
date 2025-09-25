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
use crate::share::{ApprovalStatus, TaApprovalChainBasic, TaApprovalStageBasic};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// note: keep the name consistent with the one in external:
// [Ta]ApprovalChainBasic: only contains user identity, such as UserID, Email, used for wallet initialization
// [Ta]ApprovalChain: contains user identity and approval status, used for transaction approval

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaApprovalStage {
    pub threshold: u64,
    pub status: HashMap<UserID, ApprovalStatus>,
}

impl TaApprovalStage {
    pub fn new(threshold: u64, approvers: HashSet<UserID>) -> Self {
        Self {
            threshold,
            status: approvers
                .into_iter()
                .map(|e| (e, ApprovalStatus::Pending))
                .collect(),
        }
    }

    pub fn get_threshold(&self) -> u64 {
        self.threshold
    }

    pub fn status(&self) -> &HashMap<UserID, ApprovalStatus> {
        &self.status
    }

    pub fn approvers_status_mut(&mut self) -> &mut HashMap<UserID, ApprovalStatus> {
        &mut self.status
    }

    pub fn is_approved(&self) -> bool {
        self.status
            .values()
            .filter(|s| s == &&ApprovalStatus::Approved)
            .count() as u64
            >= self.threshold
    }

    pub fn is_rejected(&self) -> bool {
        self.status.values().any(|s| s == &ApprovalStatus::Rejected)
    }

    pub fn contains(&self, user: &UserID) -> bool {
        self.status.keys().any(|k| k == user)
    }

    pub fn get_stage_overall_status(&self) -> ApprovalStatus {
        if self.is_rejected() {
            ApprovalStatus::Rejected
        } else if self.is_approved() {
            ApprovalStatus::Approved
        } else {
            ApprovalStatus::Pending
        }
    }

    pub fn match_stage(&self, other: &TaApprovalStage) -> bool {
        self.threshold == other.threshold
            && self.status.len() == other.status.len()
            && self
                .status
                .iter()
                .all(|(k, v)| other.status.get(k).map(|o| o == v).unwrap_or(false))
    }
}

// default pending status
impl From<TaApprovalStageBasic> for TaApprovalStage {
    fn from(approval_stage_basic: TaApprovalStageBasic) -> Self {
        let mut status = HashMap::new();
        for approver in approval_stage_basic.approvers() {
            status.insert(approver.clone(), ApprovalStatus::Pending);
        }
        Self {
            threshold: approval_stage_basic.threshold(),
            status,
        }
    }
}

impl From<(u64, HashMap<UserID, ApprovalStatus>)> for TaApprovalStage {
    fn from((threshold, status): (u64, HashMap<UserID, ApprovalStatus>)) -> Self {
        Self { threshold, status }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaApprovalChain(Vec<TaApprovalStage>);

impl TaApprovalChain {
    pub fn new(inner: Vec<TaApprovalStage>) -> Self {
        Self(inner)
    }

    pub fn stages(&self) -> &Vec<TaApprovalStage> {
        &self.0
    }

    pub fn take_stages(self) -> Vec<TaApprovalStage> {
        self.0
    }

    pub fn stages_mut(&mut self) -> &mut Vec<TaApprovalStage> {
        &mut self.0
    }

    fn get_first_pending_stage(&mut self) -> Result<&mut TaApprovalStage> {
        for stage in self.0.iter_mut() {
            match stage.get_stage_overall_status() {
                ApprovalStatus::Rejected => return Err(anyhow!("tx was rejected")),
                ApprovalStatus::Approved => continue,
                ApprovalStatus::Pending => return Ok(stage),
            }
        }
        Err(anyhow!("no pending stage"))
    }

    pub fn approve(&mut self, user_id: &UserID) -> Result<()> {
        let current_stage = self.get_first_pending_stage()?;
        let current_stage_status = current_stage.approvers_status_mut();
        current_stage_status.insert(user_id.clone(), ApprovalStatus::Approved);
        Ok(())
    }

    pub fn reject(&mut self, user_id: &UserID) -> Result<()> {
        let current_stage = self.get_first_pending_stage()?;
        let current_stage_status = current_stage.approvers_status_mut();
        current_stage_status.insert(user_id.clone(), ApprovalStatus::Rejected);
        Ok(())
    }

    pub fn all_approved(&self) -> bool {
        self.0
            .iter()
            .all(|stage| stage.get_stage_overall_status() == ApprovalStatus::Approved)
    }

    pub fn match_other(&self, other: &TaApprovalChain) -> bool {
        self.0.len() == other.0.len()
            && self
                .0
                .iter()
                .zip(other.0.iter())
                .all(|(a, b)| a.match_stage(b))
    }
}

impl std::ops::Deref for TaApprovalChain {
    type Target = Vec<TaApprovalStage>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<TaApprovalChainBasic> for TaApprovalChain {
    fn from(acb: TaApprovalChainBasic) -> Self {
        Self(acb.into_iter().map(|e| e.into()).collect::<Vec<_>>())
    }
}

impl std::iter::FromIterator<TaApprovalStage> for TaApprovalChain {
    fn from_iter<I: IntoIterator<Item = TaApprovalStage>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}
