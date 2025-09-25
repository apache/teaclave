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
use credential_manager::CredentialManager;
use db_manager::StorageClient;
use std::collections::HashSet;
use std::sync::Arc;
use storable::user_info::{UidToEmail, UserInfo};
use types::external::Email;
use types::share::{CkPublicKey, Role, UserID};

pub struct UserRegistry {
    db_client: Arc<StorageClient>,
}

impl UserRegistry {
    pub async fn init(db_client: Arc<StorageClient>) -> Result<Self> {
        Ok(Self { db_client })
    }

    pub async fn get_user_public_key(&self, email: Email) -> Result<CkPublicKey> {
        let user_info: UserInfo = self
            .db_client
            .get(&email)
            .await?
            .ok_or(anyhow!("user not found"))?;
        Ok(user_info.get_pub_key().clone())
    }
    pub async fn get_roles(&self, email: &Email) -> Result<Vec<Role>> {
        let user_info: UserInfo = self
            .db_client
            .get(email)
            .await?
            .ok_or(anyhow!("user not found"))?;
        Ok(user_info.get_roles().0.clone())
    }
    pub async fn get_info(&self, email: &Email) -> Result<UserInfo> {
        let user_info = self
            .db_client
            .get(email)
            .await?
            .ok_or(anyhow!("user not found"))?;
        Ok(user_info)
    }
    pub async fn get_user_info_by_id(&self, id: &UserID) -> Result<UserInfo> {
        for (_email, info) in self.db_client.list_entries::<Email, UserInfo>().await? {
            if info.get_id() == id {
                return Ok(info);
            }
        }
        bail!("user not found")
    }

    pub async fn get<T: TryFrom<UserInfo>>(&self, email: &Email) -> Result<T> {
        match self.db_client.get::<Email, UserInfo>(email).await? {
            Some(info) => {
                let role: T = info
                    .clone()
                    .try_into()
                    .map_err(|_| anyhow!("userinfo cannot be converted to role"))?;
                Ok(role)
            }
            None => bail!("user not found"),
        }
    }

    pub async fn get_all_info(&self) -> Result<Vec<UserInfo>> {
        let mut user_infos = Vec::new();
        for (_email, info) in self.db_client.list_entries::<Email, UserInfo>().await? {
            user_infos.push(info);
        }
        Ok(user_infos)
    }

    pub async fn tx_operators(&self) -> Result<Vec<UserInfo>> {
        let mut tx_operators = Vec::new();
        for (_email, info) in self.db_client.list_entries::<Email, UserInfo>().await? {
            if info.is_tx_operator() {
                tx_operators.push(info);
            }
        }
        Ok(tx_operators)
    }

    pub async fn user_exist(&self, email: &Email) -> bool {
        self.get_info(email).await.is_ok()
    }
    pub async fn add_user(
        &mut self,
        credential_manager: &CredentialManager,
        user_name: &str,
        user_email: &Email,
        user_roles: &HashSet<Role>,
    ) -> Result<UserInfo> {
        if self.user_exist(user_email).await {
            bail!("user already exists")
        }
        let public_key = credential_manager
            .generate_user_credential(user_email)
            .await?;
        // account registry db
        let userinfo = UserInfo::new(
            user_name,
            user_email.clone(),
            user_roles.clone(),
            public_key,
        )?;

        self.db_client.put(&userinfo).await?;
        let uid_to_email = UidToEmail::new(userinfo.get_id().clone(), user_email.clone());
        self.db_client.put(&uid_to_email).await?;
        Ok(userinfo)
    }

    pub async fn append_role(&mut self, email: &Email, roles: &HashSet<Role>) -> Result<UserInfo> {
        let mut info = self.get_info(email).await?;
        for role in roles.iter() {
            info.append_role((*role).clone());
            log::info!("append role {:?} to user {:?}", role, email);
        }
        log::debug!("updated user info {:?}", info);
        self.db_client.put(&info).await?;
        Ok(info.clone())
    }
}
