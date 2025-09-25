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
use std::collections::HashSet;
use types::share::{
    MultiChainAccount, TaApprovalChainBasic, TaOperatorsBasic, TeeConfig, UserID, WalletID,
};

// Wallet cmds (tls):
// - SyncWithTee
// - AddAccount (unimplemented)

// SyncWithTee
// Create a new wallet or update the wallet info in the TEE
// return LatestTeeInfo
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SyncWithTeeInput {
    pub signed_config: TeeConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LatestWalletInfo {
    pub wallet_id: WalletID,
    pub approval_chain: TaApprovalChainBasic,
    pub authorized_operators: TaOperatorsBasic,
    pub accounts: HashSet<MultiChainAccount>,
}

impl LatestWalletInfo {
    pub fn all_participants(&self) -> HashSet<&UserID> {
        let mut participants = self.approval_chain.distinct_approvers();
        participants.extend(self.authorized_operators.distinct_operators());
        participants
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SyncWithTeeOutput {
    pub latest_wallets: Vec<LatestWalletInfo>,
    pub config_version: u64,
}
