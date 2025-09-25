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

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use storable::committed_config::CommittedConfig;
use storable::synced_wallet::SyncedWalletInfo;

use task_exec::Executable;

use db_manager::StorageClient;
use proto::{LatestWalletInfo, TaCommand};
use proto::{SyncWithTeeInput, SyncWithTeeOutput};
use storable::account_to_wallet::AccountToWallet;
use storable::tee_status::OnlineDevice;
use storable::user_info::UidToEmail;
use tls_client_processing::TlsClient;
use types::external::{ApprovalStageBasic, CkAccount, Email, OperatorsBasic};
use types::share::{MultiChainAccount, MultiChainAccountId, UserID, WalletID};

pub struct SyncWalletInfoTask {
    db_client: Arc<StorageClient>,
    tls_client: Arc<RwLock<TlsClient>>,
}

impl SyncWalletInfoTask {
    pub fn new(db_client: Arc<StorageClient>, tls_client: Arc<RwLock<TlsClient>>) -> Self {
        Self {
            db_client,
            tls_client,
        }
    }

    async fn get_online_device(&self) -> Result<OnlineDevice> {
        self.db_client
            .get::<_, OnlineDevice>(&"OnlineDevice".to_string())
            .await?
            .ok_or(anyhow!("OnlineDevice not found"))
    }

    async fn need_sync(&self, online_device: &OnlineDevice) -> Result<Option<CommittedConfig>> {
        let tee_status = online_device.status();
        let device_id = online_device.device_id();

        let committed_config = match self.db_client.get::<_, CommittedConfig>(&device_id).await? {
            Some(config) => config,
            None => {
                log::warn!("CommittedConfig not found");
                return Ok(None);
            }
        };
        // if device is in "waiting-for-sync" state, we need to sync
        if tee_status.is_waiting_for_sync() {
            return Ok(Some(committed_config));
        }
        // if device is running, we need to sync if the committed config version is newer
        if tee_status.is_running() {
            let tee_config_version = tee_status.config_version()?;
            let committed_config_version = committed_config.config_version();
            if committed_config_version > tee_config_version {
                return Ok(Some(committed_config));
            }
        }
        Ok(None)
    }

    async fn init_missing_account(
        &self,
        mca: MultiChainAccount,
        wallet_id: &WalletID,
    ) -> Result<()> {
        let account_id = mca.id();
        let opt_account = self
            .db_client
            .get::<_, AccountToWallet>(&account_id)
            .await?;
        if opt_account.is_none() {
            log::info!("Account not found, initializing: {}", account_id);
            let ck_account = CkAccount::try_from(mca)?;
            let account = AccountToWallet::new(ck_account, wallet_id.clone());
            self.db_client.put(&account).await?;
        }
        Ok(())
    }

    async fn fetch_latest_info(&self) -> Result<(Vec<LatestWalletInfo>, u64)> {
        let online_device = self.get_online_device().await?;
        if let Some(committed_config) = self.need_sync(&online_device).await? {
            let output: SyncWithTeeOutput = self.tls_client.write().unwrap().invoke(
                SyncWithTeeInput {
                    signed_config: committed_config.signed_config,
                },
                TaCommand::SyncWithTee,
            )?;
            Ok((output.latest_wallets, output.config_version))
        } else {
            bail!("No need to sync");
        }
    }

    async fn store_synced_wallet_info(
        &self,
        wallet_info: LatestWalletInfo,
        config_version: u64,
        uid_to_email_cache: &mut HashMap<UserID, Email>,
    ) -> Result<()> {
        for uid in wallet_info.all_participants() {
            if !uid_to_email_cache.contains_key(uid) {
                if let Some(info) = self.db_client.get::<_, UidToEmail>(uid).await? {
                    uid_to_email_cache.insert(uid.clone(), info.email());
                } else {
                    log::error!("Failed to get email for uid: {:?}", uid);
                    return Err(anyhow::anyhow!("Failed to get email for uid: {:?}", uid));
                }
            }
        }

        // TaApprovalChainBasic cannot be transferred to the ApprovalChainBasic by "From" trait
        // because we need the uid_to_email mapping
        let approval_chain = wallet_info
            .approval_chain
            .into_iter()
            .map(|stage| {
                ApprovalStageBasic::new(
                    stage.threshold(),
                    stage
                        .approvers()
                        .iter()
                        .map(|uid| uid_to_email_cache.get(uid).unwrap().clone())
                        .collect(),
                )
            })
            .collect();

        let authorized_operators = OperatorsBasic::new(
            wallet_info
                .authorized_operators
                .iter()
                .map(|uid| uid_to_email_cache.get(uid).unwrap().clone())
                .collect(),
        );

        let accounts = wallet_info.accounts;
        let account_ids = accounts.iter().map(MultiChainAccountId::from_mca).collect();

        for account in accounts.into_iter() {
            self.init_missing_account(account, &wallet_info.wallet_id)
                .await?;
        }

        let synced_wallet_info = SyncedWalletInfo::new(
            wallet_info.wallet_id,
            approval_chain,
            authorized_operators,
            account_ids,
            config_version,
        );
        self.db_client.put(&synced_wallet_info).await
    }

    async fn inner_exec(&self) -> Result<()> {
        let (latest_info_list, config_version) = self.fetch_latest_info().await?;
        let mut uid_to_email_cache = HashMap::new();
        for wallet_info in latest_info_list {
            self.store_synced_wallet_info(wallet_info, config_version, &mut uid_to_email_cache)
                .await?;
        }
        {
            let mut online_device = self
                .db_client
                .get::<_, OnlineDevice>(&"OnlineDevice".to_string())
                .await?
                .ok_or(anyhow!("OnlineDevice not found"))?;
            online_device.set_running(config_version);
            self.db_client.put(&online_device).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Executable for SyncWalletInfoTask {
    async fn exec(&self) {
        if let Err(e) = self.inner_exec().await {
            log::warn!("Failed to sync wallet info: {:?}", e);
        }
    }
}
