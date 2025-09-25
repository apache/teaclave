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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use types::share::{
    CkEncryptedPayload, CkPublicKey, CkSignature, DeviceID, TeeOnlineStatus, WalletID,
};

// GetTeeStatus
#[derive(Serialize, Deserialize, Debug)]
pub struct GetTeeStatusInput {}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetTeeStatusOutput {
    pub device_id: DeviceID,
    pub tee_status: TeeOnlineStatus,
}

// Backup commands (local):
// - InitBoard
// - BackupWallet
// - RestoreWallet

// InitBoard
#[derive(Serialize, Deserialize, Debug)]
pub struct InitBoardInput {}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitBoardOutput {
    pub device_id: DeviceID,
    pub signing_pubkey: CkPublicKey,
    pub backup_pubkey: CkPublicKey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BackupWalletInput {
    pub backup_to_device_id: DeviceID,
    pub backup_to_device_pubkey: CkPublicKey,
    pub target_wallets: HashSet<WalletID>,
    pub signature: Option<CkSignature>, // signature of requests
}

impl BackupWalletInput {
    pub fn new_unsigned(
        backup_to_device_id: DeviceID,
        backup_to_device_pubkey: CkPublicKey,
        target_wallets: HashSet<WalletID>,
    ) -> Self {
        Self {
            backup_to_device_id,
            backup_to_device_pubkey,
            target_wallets,
            signature: None,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.extend(bincode::serialize(&self.backup_to_device_id)?);
        bytes.extend(bincode::serialize(&self.backup_to_device_pubkey)?);
        let mut target_wallets: Vec<WalletID> = self.target_wallets.iter().cloned().collect();
        target_wallets.sort();
        bytes.extend(bincode::serialize(&target_wallets)?);
        Ok(bytes)
    }

    pub fn set_signature(&mut self, signature: CkSignature) {
        self.signature = Some(signature);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BackupWalletOutput {
    pub backup_to_device: DeviceID,
    pub wallet_ids: HashSet<WalletID>, // backup wallets
    pub encrypted: CkEncryptedPayload,
}

// RestoreWallet
#[derive(Serialize, Deserialize, Debug)]
pub struct RestoreWalletInput {
    pub backup_to_device: DeviceID,
    pub wallet_ids: HashSet<WalletID>, // backup wallets
    pub encrypted: CkEncryptedPayload,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RestoreWalletOutput {
    pub wallet_ids: HashSet<WalletID>, // successfully restored wallets
}
