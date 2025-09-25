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

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use storable::console::ConsoleWalletInfo;
use types::external::{ApprovalChainBasic, OperatorsBasic, ViewersBasic};
use types::share::{AccountId, DeviceID, WalletID};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleWalletInfoForClient {
    pub wallet_id: WalletID,
    pub wallet_name: String,
    pub account_address: Option<AccountId>,
    pub created_by_device: DeviceID,
    pub backed_up_by_devices: HashSet<DeviceID>,
    pub approval_chain_basic: ApprovalChainBasic,
    pub authorized_operators: OperatorsBasic,
    pub viewers: ViewersBasic,
    pub update_time: u64,
}

impl ConsoleWalletInfoForClient {
    pub fn new(wallet_info_basic: ConsoleWalletInfo, account_address: Option<AccountId>) -> Self {
        Self {
            wallet_id: wallet_info_basic.wallet_id,
            wallet_name: wallet_info_basic.wallet_name,
            account_address,
            created_by_device: wallet_info_basic.created_by_device,
            backed_up_by_devices: wallet_info_basic.backed_up_by_devices,
            approval_chain_basic: wallet_info_basic.approval_chain_basic,
            authorized_operators: wallet_info_basic.authorized_operators,
            viewers: wallet_info_basic.viewers,
            update_time: wallet_info_basic.update_time,
        }
    }
}
