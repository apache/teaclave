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

use crate::share::{CkSignature, TaApprovalChainBasic, TaOperatorsBasic, TaUserInfo, WalletID};
use anyhow::Result;
use serde::{Deserialize, Serialize};

// SyncWithTee Command Input
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaWalletInfo {
    pub wallet_id: WalletID,
    pub approval_chain: TaApprovalChainBasic,
    pub authorized_operators: TaOperatorsBasic,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TeeConfig {
    pub user_registry: Vec<TaUserInfo>, // CkHash: hash of public key
    pub wallets: Vec<TaWalletInfo>,     // each wallet info
    pub config_version: u64,
    pub signature: Option<CkSignature>, // signature of (user_registry + wallets + config_version)
}

impl TeeConfig {
    pub fn new(
        user_registry: Vec<TaUserInfo>,
        wallets: Vec<TaWalletInfo>,
        config_version: u64,
    ) -> Self {
        Self {
            user_registry,
            wallets,
            config_version,
            signature: None,
        }
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.extend(bincode::serialize(&self.user_registry)?);
        bytes.extend(bincode::serialize(&self.wallets)?);
        bytes.extend(bincode::serialize(&self.config_version)?);
        Ok(bytes)
    }

    pub fn set_signature(&mut self, signature: CkSignature) {
        self.signature = Some(signature);
    }
}
