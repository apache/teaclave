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

use crate::console::ConsoleWalletInfo;
use crate::Storable;

use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};
use types::external::Email;
use types::share::{CkPublicKey, DeviceID, WalletID};

// simplified structure for user input/output
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceInfoBasic {
    pub owner: Email,
    pub id: DeviceID,
    pub register_time: u64,
    pub created_wallets: HashSet<WalletID>,
    pub backed_up_wallets: HashSet<WalletID>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupDeviceInfoBasic {
    pub device_id: DeviceID,
    pub wallet_id_list: Vec<WalletID>, // wallets to be backed up into this device
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceInfo {
    owner: Email,
    id: DeviceID,
    signing_pubkey: CkPublicKey,
    backup_pubkey: CkPublicKey,
    cert: Vec<u8>,
    register_time: u64,
    created_wallets: HashSet<WalletID>,
    backed_up_wallets: HashSet<WalletID>,
}

impl Storable<DeviceID> for DeviceInfo {
    fn unique_id(&self) -> DeviceID {
        self.id.clone()
    }
}

impl DeviceInfo {
    pub fn new(
        owner: Email,
        id: DeviceID,
        signing_pubkey: CkPublicKey,
        backup_pubkey: CkPublicKey,
        cert: Vec<u8>,
        created_wallets: HashSet<WalletID>,
        backed_up_wallets: HashSet<WalletID>,
    ) -> Self {
        Self {
            owner,
            id,
            signing_pubkey,
            backup_pubkey,
            cert,
            register_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            created_wallets,
            backed_up_wallets,
        }
    }
    pub fn owner(&self) -> &Email {
        &self.owner
    }
    pub fn id(&self) -> &DeviceID {
        &self.id
    }
    pub fn signing_pubkey(&self) -> &CkPublicKey {
        &self.signing_pubkey
    }
    pub fn backup_pubkey(&self) -> &CkPublicKey {
        &self.backup_pubkey
    }
    pub fn cert(&self) -> &Vec<u8> {
        &self.cert
    }
    pub fn set_cert(&mut self, cert: Vec<u8>) {
        self.cert = cert;
    }
    pub fn register_time(&self) -> u64 {
        self.register_time
    }
    pub fn created_wallets(&self) -> &HashSet<WalletID> {
        &self.created_wallets
    }
    pub fn backed_up_wallets(&self) -> &HashSet<WalletID> {
        &self.backed_up_wallets
    }
    pub fn add_owned_wallet(&mut self, wallet_info_basic: &ConsoleWalletInfo) -> Result<()> {
        ensure!(
            !self.created_wallets.contains(wallet_info_basic.id()),
            "DeviceInfo::add_owned_wallet(): wallet {:?} already exists in created_wallets",
            &wallet_info_basic.id()
        );
        ensure!(
            !self.backed_up_wallets.contains(wallet_info_basic.id()),
            "DeviceInfo::add_owned_wallet(): wallet already exist in backed_up_wallets, wallet_id: {:?}",
            &wallet_info_basic.id()
        );
        self.created_wallets.insert(wallet_info_basic.id().clone());
        Ok(())
    }
    pub fn add_backup_wallet(&mut self, wallet_id: &WalletID) -> Result<()> {
        if self.backed_up_wallets.contains(wallet_id) {
            log::info!(
                "DeviceInfo::add_backup_wallet(): wallet {:?} already exists in backed_up_wallets, skip",
                &wallet_id
            );
            return Ok(());
        }
        self.backed_up_wallets.insert(wallet_id.clone());
        Ok(())
    }
    pub fn existing_wallets_num(&self) -> usize {
        self.created_wallets.len() + self.backed_up_wallets.len()
    }
    pub fn wallet_id_list(&self) -> Vec<WalletID> {
        let mut wallet_id_list = self.created_wallets.clone();
        wallet_id_list.extend(self.backed_up_wallets.clone());
        wallet_id_list.into_iter().collect()
    }
    pub fn remove_owned_wallet(&mut self, wallet_id: &WalletID) -> Result<()> {
        ensure!(
            self.created_wallets.contains(wallet_id),
            "DeviceInfo::remove_owned_wallet(): wallet {:?} does not exist in created_wallets",
            &wallet_id
        );
        self.created_wallets.remove(wallet_id);
        Ok(())
    }
}

impl From<DeviceInfo> for DeviceInfoBasic {
    fn from(device_info: DeviceInfo) -> Self {
        Self {
            owner: device_info.owner,
            id: device_info.id,
            register_time: device_info.register_time,
            created_wallets: device_info.created_wallets,
            backed_up_wallets: device_info.backed_up_wallets,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeviceIndex {
    email: Email,
    device_list: HashSet<DeviceID>,
}

impl Storable<Email> for DeviceIndex {
    fn unique_id(&self) -> Email {
        self.email.clone()
    }
}

impl DeviceIndex {
    pub fn new(email: Email, device_list: HashSet<DeviceID>) -> Self {
        Self { email, device_list }
    }
    pub fn _email(&self) -> &Email {
        &self.email
    }
    pub fn device_list(&self) -> &HashSet<DeviceID> {
        &self.device_list
    }
}
