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

use crate::synced_wallet::SyncedWalletInfo;
use crate::Storable;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::SystemTime;
use types::external::{ApprovalChainBasic, Email, OperatorsBasic, ViewersBasic};
use types::share::{AccountId, DeviceID, TaWalletInfo, WalletID};

pub const MAX_WALLET_NUM_PER_DEVICE: usize = 10;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleWalletInfo {
    pub wallet_id: WalletID,
    pub wallet_name: String,
    pub created_by_device: DeviceID,
    pub backed_up_by_devices: HashSet<DeviceID>,
    pub approval_chain_basic: ApprovalChainBasic,
    pub authorized_operators: OperatorsBasic,
    pub viewers: ViewersBasic,
    pub update_time: u64,
}

impl Storable<WalletID> for ConsoleWalletInfo {
    fn unique_id(&self) -> WalletID {
        self.wallet_id.clone()
    }
}

impl ConsoleWalletInfo {
    pub fn new(
        created_by_device: DeviceID,
        wallet_name: String,
        approval_chain_basic: ApprovalChainBasic,
        authorized_operators: OperatorsBasic,
        viewers: ViewersBasic,
    ) -> Result<Self> {
        Ok(Self {
            wallet_name,
            created_by_device,
            backed_up_by_devices: HashSet::new(),
            wallet_id: WalletID::new()?,
            approval_chain_basic,
            authorized_operators,
            viewers,
            update_time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs(),
        })
    }
    pub fn distinct_participants(&self) -> HashSet<&Email> {
        let mut users = self.approval_chain_basic.distinct_approvers();
        users.extend(self.authorized_operators.distinct_operators());
        users
    }
    pub fn id(&self) -> &WalletID {
        &self.wallet_id
    }
    pub fn approval_chain_basic(&self) -> &ApprovalChainBasic {
        &self.approval_chain_basic
    }
    pub fn authorized_operators(&self) -> &OperatorsBasic {
        &self.authorized_operators
    }
    pub fn can_view(&self, email: &Email) -> bool {
        self.viewers.contains(email)
    }
    pub fn viewers(&self) -> &ViewersBasic {
        &self.viewers
    }
    pub fn update_info(
        &mut self,
        wallet_name: &str,
        approval_chain_basic: &ApprovalChainBasic,
        authorized_operators: &OperatorsBasic,
        viewers: &ViewersBasic,
    ) {
        self.wallet_name = wallet_name.to_owned();
        self.viewers = viewers.to_owned();
        self.approval_chain_basic = approval_chain_basic.to_owned();
        self.authorized_operators = authorized_operators.to_owned();
        let update_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.update_time = update_time;
        log::info!("update info_update_time to {}", update_time);
    }
    pub fn revert_info(&mut self, synced_wallet_info: &SyncedWalletInfo) {
        self.approval_chain_basic = synced_wallet_info.approval_chain.clone();
        self.authorized_operators = synced_wallet_info.authorized_operators.clone();
        self.update_time = synced_wallet_info.config_version;
        log::info!(
            "revert console wallet info to config version: {}",
            self.update_time
        );
    }
    pub fn add_backup_device(&mut self, device_id: DeviceID) {
        self.backed_up_by_devices.insert(device_id);
    }
}

impl From<ConsoleWalletInfo> for TaWalletInfo {
    fn from(cw: ConsoleWalletInfo) -> Self {
        Self {
            wallet_id: cw.wallet_id,
            approval_chain: cw.approval_chain_basic.into(),
            authorized_operators: cw.authorized_operators.into(),
        }
    }
}

// simplified structures for user input/output
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateConsoleWalletInfo {
    pub wallet_name: String,
    pub approval_chain_basic: ApprovalChainBasic,
    pub authorized_operators: OperatorsBasic,
    pub viewers: ViewersBasic,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConsoleWalletInfo {
    pub wallet_id: WalletID,
    pub wallet_name: String,
    pub approval_chain_basic: ApprovalChainBasic,
    pub authorized_operators: OperatorsBasic,
    pub viewers: ViewersBasic,
}

impl From<ConsoleWalletInfo> for UpdateConsoleWalletInfo {
    fn from(wallet_info_basic: ConsoleWalletInfo) -> Self {
        Self {
            wallet_id: wallet_info_basic.wallet_id,
            wallet_name: wallet_info_basic.wallet_name,
            approval_chain_basic: wallet_info_basic.approval_chain_basic,
            authorized_operators: wallet_info_basic.authorized_operators,
            viewers: wallet_info_basic.viewers,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct WalletAccountInfo {
    pub wallet_id: WalletID,
    pub account_address: AccountId,
}

impl Storable<WalletID> for WalletAccountInfo {
    fn unique_id(&self) -> WalletID {
        self.wallet_id.clone()
    }
}

impl WalletAccountInfo {
    pub fn new(wallet_id: WalletID, account_address: AccountId) -> Self {
        Self {
            wallet_id,
            account_address,
        }
    }
}
