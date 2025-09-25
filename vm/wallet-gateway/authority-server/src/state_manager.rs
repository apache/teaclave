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

use anyhow::{anyhow, bail, ensure, Result};
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::SystemTime;
use storable::committed_config::CommittedConfig;
use storable::synced_wallet::SyncedWalletInfo;
use storable::tee_status::OnlineDevice;
use tokio::sync::RwLock;
use url::Url;

use credential_manager::CredentialManager;
use storable::console::{ConsoleWalletInfo, CreateConsoleWalletInfo, UpdateConsoleWalletInfo};
use storable::device_info::{BackupDeviceInfoBasic, DeviceInfoBasic};
use storable::user_info::{User, UserInfo};

use authority::{DeviceRegistry, UserRegistry};
use db_manager::{DBCompatibleClient, LocalServiceClient, StorageClient};
use proto::InitBoardOutput;
use types::external::Email;
use types::share::{AccountId, DeviceID, Role, TeeOnlineStatus, UserID, WalletID};

use crate::auth_data::AuthData;
use crate::config::AuthorityConfig;
use crate::console_user::ConsoleUserInfo;
use crate::file_utils::{export_file, ExportedFileType};
use crate::UserBasicInfo;

pub struct SharedStateManager {
    config: Arc<AuthorityConfig>,
    user_registry: Arc<RwLock<UserRegistry>>,
    device_registry: Arc<RwLock<DeviceRegistry>>,
    credential_manager: Arc<CredentialManager>,
    db_client: Arc<StorageClient>,
}

impl SharedStateManager {
    pub async fn new(config: Arc<AuthorityConfig>) -> Result<Self> {
        let db_client = Arc::new(StorageClient::new(Box::new(
            LocalServiceClient::init(config.db_server_url.as_str(), None).await?,
        )));
        let user_registry = Arc::new(RwLock::new(UserRegistry::init(db_client.clone()).await?));
        let device_registry = Arc::new(RwLock::new(DeviceRegistry::init(db_client.clone()).await?));
        let credential_manager = Arc::new(
            CredentialManager::init(
                db_client.clone(),
                config.cert_path.to_str().unwrap_or_default(),
            )
            .await?,
        );
        Ok(Self {
            config,
            user_registry,
            device_registry,
            credential_manager,
            db_client,
        })
    }

    // /user/register
    pub async fn register_user(&self, user_basic_info: &UserBasicInfo) -> Result<UserInfo> {
        self.user_registry
            .write()
            .await
            .add_user(
                &self.credential_manager,
                &user_basic_info.name,
                &user_basic_info.email,
                &user_basic_info.roles,
            )
            .await
    }
    // /user/role/append
    pub async fn append_role(&self, user_email: &Email, roles: &HashSet<Role>) -> Result<UserInfo> {
        self.user_registry
            .write()
            .await
            .append_role(user_email, roles)
            .await
    }
    // /user/info/get
    pub async fn get_user_info(&self, user_email: &Email) -> Result<UserInfo> {
        self.user_registry.read().await.get_info(user_email).await
    }
    // /user/info/get-all
    pub async fn get_all_user_info(&self) -> Result<Vec<UserInfo>> {
        self.user_registry.read().await.get_all_info().await
    }

    // /user/info/get_by_id
    pub async fn get_user_info_by_id(&self, user_id: &UserID) -> Result<UserInfo> {
        self.user_registry
            .read()
            .await
            .get_user_info_by_id(user_id)
            .await
    }

    // /device/register
    pub async fn register_device(
        &self,
        user_email: &Email,
        device_pubkeys: &str,
    ) -> Result<DeviceInfoBasic> {
        // base64 decode
        let decoded_pubkeys = general_purpose::STANDARD.decode(device_pubkeys)?;
        // deserialize
        let device_pubkeys: InitBoardOutput = bincode::deserialize(&decoded_pubkeys)?;
        log::debug!("device_pubkeys: {:?}", device_pubkeys);
        self.device_registry
            .write()
            .await
            .register_device(
                &self.credential_manager,
                user_email,
                device_pubkeys.signing_pubkey,
                device_pubkeys.backup_pubkey,
            )
            .await
    }

    // admin or associated device owner can operate on the device
    pub async fn _user_can_operate_on_device(&self, user: &User, device_id: &DeviceID) -> bool {
        if user.get_info().is_admin() {
            return true;
        }
        self.device_registry
            .read()
            .await
            .device_exist_for_user(user.get_info().get_email(), device_id)
            .await
    }

    // /device/cert/export
    pub async fn export_device_cert(&self, device_id: &DeviceID) -> Result<Url> {
        let device_cert = self
            .device_registry
            .read()
            .await
            .export_device_cert(device_id)
            .await?;
        let id: String = device_id.clone().into();
        let url = export_file(
            &self.config.export_base_path,
            &self.config.export_base_url,
            id,
            &ExportedFileType::Cert,
            &device_cert,
            None,
        )?;
        log::info!("exported device cert: {}", url);
        Ok(url)
    }
    // /device/cert/refresh
    pub async fn refresh_device_cert(&self, device_id: &DeviceID) -> Result<Url> {
        let device_cert = self
            .device_registry
            .write()
            .await
            .refresh_device_cert(&self.credential_manager, device_id)
            .await?;
        let id: String = device_id.clone().into();
        let url = export_file(
            &self.config.export_base_path,
            &self.config.export_base_url,
            id,
            &ExportedFileType::Cert,
            &device_cert,
            None,
        )?;
        log::info!("refreshed device cert: {}", url);
        Ok(url)
    }
    // /device/authorize
    pub async fn authorize_backup_device(
        &self,
        from_device: &DeviceID,
        backup_info: &BackupDeviceInfoBasic,
    ) -> Result<Url> {
        let (serialized_input, timestamp) = self
            .device_registry
            .read()
            .await
            .export_backup_device_list(&self.credential_manager, from_device, backup_info)
            .await?;
        let from_device_id: String = from_device.clone().into();
        let url = export_file(
            &self.config.export_base_path,
            &self.config.export_base_url,
            from_device_id,
            &ExportedFileType::SignedBackupList,
            &serialized_input,
            Some(timestamp),
        )?;
        log::info!("authorized backup device: {}", url);
        Ok(url)
    }
    // /device/info/get
    pub async fn get_device_info(&self, device_id: &DeviceID) -> Result<DeviceInfoBasic> {
        match self
            .device_registry
            .read()
            .await
            .get_device_info(device_id)
            .await?
        {
            Some(info) => Ok(info),
            None => bail!("Device not found"),
        }
    }
    // /device/current-online
    pub async fn get_current_online_device(&self) -> Result<DeviceID> {
        match self
            .db_client
            .get::<_, OnlineDevice>(&"OnlineDevice".to_string())
            .await?
        {
            Some(info) => Ok(info.device_id()),
            None => bail!("No online device found"),
        }
    }

    pub async fn get_wallet_address(&self, wallet_id: &WalletID) -> Result<Option<AccountId>> {
        // get account address from db
        let synced_wallet_info = match self.db_client.get::<_, SyncedWalletInfo>(wallet_id).await? {
            Some(info) => info,
            None => {
                log::warn!("Wallet not found in db: {:?}", wallet_id);
                return Ok(None);
            }
        };

        Ok(synced_wallet_info
            .accounts
            .into_iter()
            .next()
            .map(|mca| mca.id().clone()))
    }

    // /wallet/register
    pub async fn register_wallets(
        &self,
        device_id: &DeviceID,
        create_wallet_info_list: &Vec<CreateConsoleWalletInfo>,
    ) -> Result<Vec<ConsoleWalletInfo>> {
        let mut wallet_info_basic_vec = Vec::new();
        for create_wallet_info_basic in create_wallet_info_list {
            let wallet_info_basic = ConsoleWalletInfo::new(
                device_id.clone(),
                create_wallet_info_basic.wallet_name.clone(),
                create_wallet_info_basic.approval_chain_basic.clone(),
                create_wallet_info_basic.authorized_operators.clone(),
                create_wallet_info_basic.viewers.clone(),
            )?;
            // ensure all user exist
            for user_email in wallet_info_basic.distinct_participants() {
                ensure!(
                    self.user_registry.read().await.user_exist(user_email).await,
                    "user not exist"
                );
            }
            let registered_wallet_id = self
                .device_registry
                .write()
                .await
                .register_wallet(self.user_registry.clone(), device_id, &wallet_info_basic)
                .await?;
            log::info!(
                "Register wallet successfully, wallet id: {:?}",
                registered_wallet_id
            );
            wallet_info_basic_vec.push(wallet_info_basic);
        }
        Ok(wallet_info_basic_vec)
    }

    pub async fn wallet_create_with_id(
        &self,
        device_id: &DeviceID,
        create_wallet_info_list: &Vec<UpdateConsoleWalletInfo>,
    ) -> Result<Vec<ConsoleWalletInfo>> {
        let update_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut wallet_info_basic_vec = Vec::new();
        for create_wallet_info_basic in create_wallet_info_list {
            let wallet_info_basic = ConsoleWalletInfo {
                wallet_id: create_wallet_info_basic.wallet_id.clone(),
                wallet_name: create_wallet_info_basic.wallet_name.clone(),
                created_by_device: device_id.clone(),
                approval_chain_basic: create_wallet_info_basic.approval_chain_basic.clone(),
                authorized_operators: create_wallet_info_basic.authorized_operators.clone(),
                viewers: create_wallet_info_basic.viewers.clone(),
                backed_up_by_devices: HashSet::new(),
                update_time,
            };

            // ensure all user exist
            for user_email in wallet_info_basic.distinct_participants() {
                ensure!(
                    self.user_registry.read().await.user_exist(user_email).await,
                    "user not exist"
                );
            }
            let registered_wallet_id = self
                .device_registry
                .write()
                .await
                .register_wallet(self.user_registry.clone(), device_id, &wallet_info_basic)
                .await?;
            log::info!(
                "Register wallet successfully, wallet id: {:?}",
                registered_wallet_id
            );
            wallet_info_basic_vec.push(wallet_info_basic);
        }
        Ok(wallet_info_basic_vec)
    }

    // /wallet/info/update
    pub async fn update_wallet_info(
        &self,
        device_id: &DeviceID,
        update_wallet_info_list: &Vec<UpdateConsoleWalletInfo>,
    ) -> Result<Vec<ConsoleWalletInfo>> {
        let mut updated_wallet_list = Vec::new();

        let device_info: DeviceInfoBasic = match self
            .device_registry
            .read()
            .await
            .get_device_info(device_id)
            .await?
        {
            Some(info) => info,
            None => bail!("Device not found"),
        };
        let device_owned_wallets = device_info.created_wallets;

        for update_wallet_info_basic in update_wallet_info_list {
            // load wallet info
            let mut wallet_info_basic = match self
                .device_registry
                .read()
                .await
                .get_wallet_info(&update_wallet_info_basic.wallet_id)
                .await?
            {
                Some(info) => info,
                None => bail!("Wallet not found"),
            };
            // check correlation between device and wallet
            ensure!(
                &wallet_info_basic.created_by_device == device_id,
                "DeviceRegistry::update_wallet_info(): wallet not owned by device"
            );
            ensure!(
                device_owned_wallets.contains(wallet_info_basic.id()),
                "DeviceRegistry::update_wallet_info(): wallet not owned by device"
            );
            // updated wallet info
            wallet_info_basic.update_info(
                &update_wallet_info_basic.wallet_name,
                &update_wallet_info_basic.approval_chain_basic,
                &update_wallet_info_basic.authorized_operators,
                &update_wallet_info_basic.viewers,
            );

            // ensure all user of wallet info exist
            for user_email in wallet_info_basic.distinct_participants() {
                ensure!(
                    self.user_registry.read().await.user_exist(user_email).await,
                    "user: {:?} not exist",
                    &user_email
                );
            }
            // save updated wallet info
            self.device_registry
                .read()
                .await
                .save_wallet_info(&wallet_info_basic)
                .await?;
            updated_wallet_list.push(wallet_info_basic);
        }
        Ok(updated_wallet_list)
    }
    // /wallet/info/get
    pub async fn get_wallet_info(&self, wallet_id: &WalletID) -> Result<ConsoleWalletInfo> {
        match self
            .device_registry
            .read()
            .await
            .get_wallet_info(wallet_id)
            .await?
        {
            Some(info) => Ok(info),
            None => bail!("Wallet not found"),
        }
    }
    // /wallet/info/get-all
    pub async fn get_all_wallet_info(
        &self,
        device_id: &DeviceID,
    ) -> Result<Vec<ConsoleWalletInfo>> {
        let mut wallet_info_basic_vec = Vec::new();
        // get wallet id
        let wallet_id_vec = self
            .device_registry
            .read()
            .await
            .get_wallet_id_associated_with_device(device_id)
            .await?;
        // get wallet info
        for wallet_id in wallet_id_vec {
            let wallet_info_basic = match self
                .device_registry
                .read()
                .await
                .get_wallet_info(&wallet_id)
                .await?
            {
                Some(info) => info,
                None => bail!("Wallet not found"),
            };
            wallet_info_basic_vec.push(wallet_info_basic);
        }
        Ok(wallet_info_basic_vec)
    }
    pub async fn revert_wallet_info(&self, wallet_id: &WalletID) -> Result<()> {
        let mut console_wallet_info = self
            .db_client
            .get::<_, ConsoleWalletInfo>(wallet_id)
            .await?
            .ok_or(anyhow!("console_wallet_info not found for {:?}", wallet_id))?;
        if let Some(synced_wallet_info) =
            self.db_client.get::<_, SyncedWalletInfo>(wallet_id).await?
        {
            console_wallet_info.revert_info(&synced_wallet_info);
            self.db_client.put(&console_wallet_info).await
        } else {
            ensure!(
                self.get_wallet_address(wallet_id).await?.is_none(),
                "console_wallet_info has been created in TEE before, but not synced successfully last time"
            );
            log::warn!(
                "synced_wallet_info not found for {:?}, remove the pending wallet",
                wallet_id
            );
            self.device_registry
                .write()
                .await
                .remove_wallet(wallet_id)
                .await
        }
    }

    pub async fn authenticate_console_user<U: TryFrom<ConsoleUserInfo>>(
        &self,
        auth_data: &AuthData,
    ) -> Result<U> {
        let email = auth_data
            .get_authorized_email(&self.config.auth_service_url)
            .await?;
        ConsoleUserInfo::new(email)
            .try_into()
            .map_err(|_| anyhow!("User is not console user"))
    }

    pub async fn authenticate(&self, auth_data: &AuthData) -> Result<Email> {
        auth_data
            .get_authorized_email(&self.config.auth_service_url)
            .await
    }

    // /device/status
    pub async fn get_tee_status(&self, device_id: &DeviceID) -> Result<Option<TeeOnlineStatus>> {
        let online_device = self
            .db_client
            .get::<_, OnlineDevice>(&"OnlineDevice".to_string())
            .await?;
        match online_device {
            Some(info) => {
                if &info.device_id() == device_id {
                    Ok(Some(info.status().clone()))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    // /device/sync
    // sync_config will generate a CommittedConfig and store it in db
    pub async fn sync_config(&self, device_id: &DeviceID) -> Result<()> {
        let (signed_config, timestamp) = self
            .device_registry
            .read()
            .await
            .export_wallet_info(
                &self.credential_manager,
                self.user_registry.clone(),
                device_id,
            )
            .await?;
        // export to file for backup
        let _url = export_file(
            &self.config.export_base_path,
            &self.config.export_base_url,
            device_id.clone(),
            &ExportedFileType::WalletInfo,
            &bincode::serialize(&signed_config)?,
            Some(timestamp),
        )?;
        log::info!(
            "sync_config(): wallet info exported to file with config version {}",
            timestamp
        );
        let committed_config = CommittedConfig {
            device_id: device_id.clone(),
            signed_config,
            timestamp,
        };
        self.db_client.put(&committed_config).await
    }
}
