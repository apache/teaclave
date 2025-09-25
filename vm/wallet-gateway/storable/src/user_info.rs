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

use crate::Storable;
use anyhow::{bail, ensure, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::convert::TryFrom;
use std::hash::Hash;
use types::external::Email;
use types::share::{CkPublicKey, Role, RoleSet, UserID};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    name: String,
    email: Email,
    roles: RoleSet,
    pub_key: CkPublicKey,
    user_id: UserID,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct UidToEmail {
    user_id: UserID,
    email: Email,
}

impl UidToEmail {
    pub fn new(user_id: UserID, email: Email) -> Self {
        Self { user_id, email }
    }

    pub fn email(&self) -> Email {
        self.email.clone()
    }
}

impl Storable<UserID> for UidToEmail {
    fn unique_id(&self) -> UserID {
        self.user_id.clone()
    }
}

impl UserInfo {
    pub fn new(
        name: &str,
        email: Email,
        roles: HashSet<Role>,
        pub_key: CkPublicKey,
    ) -> Result<Self> {
        Ok(Self {
            name: name.to_string(),
            email: email.clone(),
            roles: RoleSet(roles.into_iter().collect()),
            pub_key: pub_key.clone(),
            user_id: UserID::from(email),
        })
    }

    pub fn get_email(&self) -> &Email {
        &self.email
    }

    pub fn get_roles(&self) -> &RoleSet {
        &self.roles
    }

    pub fn get_pub_key(&self) -> &CkPublicKey {
        &self.pub_key
    }

    pub fn get_id(&self) -> &UserID {
        &self.user_id
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn is_admin(&self) -> bool {
        self.roles.0.contains(&Role::Admin)
    }

    pub fn is_approver(&self) -> bool {
        self.roles.0.contains(&Role::Approver)
    }

    pub fn is_tx_operator(&self) -> bool {
        self.roles.0.contains(&Role::TxOperator)
    }

    pub fn is_viewer(&self) -> bool {
        self.roles.0.contains(&Role::Viewer)
    }

    pub fn append_role(&mut self, role: Role) {
        self.roles.insert(role);
    }
}

#[cfg(target_arch = "x86_64")]
impl Storable<Email> for UserInfo {
    fn unique_id(&self) -> Email {
        self.email.clone()
    }
}

impl TryFrom<UserInfo> for User {
    type Error = anyhow::Error;

    fn try_from(user_info: UserInfo) -> Result<Self> {
        if user_info.is_tx_operator() {
            Ok(User::TxOperator(TxOperator(user_info)))
        } else if user_info.is_approver() {
            Ok(User::Approver(Approver(user_info)))
        } else if user_info.is_admin() {
            Ok(User::Admin(Admin(user_info)))
        } else if user_info.is_viewer() {
            Ok(User::Viewer(Viewer(user_info)))
        } else {
            bail!("unknown role")
        }
    }
}

impl TryFrom<UserInfo> for Admin {
    type Error = anyhow::Error;

    fn try_from(user_info: UserInfo) -> Result<Self> {
        ensure!(user_info.is_admin(), "not admin");
        let admin = Admin(user_info);
        Ok(admin)
    }
}

impl TryFrom<UserInfo> for Approver {
    type Error = anyhow::Error;

    fn try_from(user_info: UserInfo) -> Result<Self> {
        ensure!(user_info.is_approver(), "not approver");
        let approver = Approver(user_info);
        Ok(approver)
    }
}

impl TryFrom<UserInfo> for TxOperator {
    type Error = anyhow::Error;

    fn try_from(user_info: UserInfo) -> Result<Self> {
        ensure!(user_info.is_tx_operator(), "not tx operator");
        let tx_operator = TxOperator(user_info);
        Ok(tx_operator)
    }
}

impl TryFrom<UserInfo> for Viewer {
    type Error = anyhow::Error;

    fn try_from(user_info: UserInfo) -> Result<Self> {
        ensure!(user_info.is_viewer(), "not viewer");
        let viewer = Viewer(user_info);
        Ok(viewer)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Admin(pub UserInfo);

impl std::ops::Deref for Admin {
    type Target = UserInfo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Hash)]
pub struct Approver(pub UserInfo);

impl std::ops::Deref for Approver {
    type Target = UserInfo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Hash)]
pub struct TxOperator(pub UserInfo);

impl std::ops::Deref for TxOperator {
    type Target = UserInfo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Hash)]
pub struct Viewer(pub UserInfo);

impl std::ops::Deref for Viewer {
    type Target = UserInfo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum User {
    Admin(Admin),
    Approver(Approver),
    TxOperator(TxOperator),
    Viewer(Viewer),
}

impl User {
    pub fn get_info(&self) -> &UserInfo {
        match self {
            User::Admin(admin) => &admin.0,
            User::Approver(approver) => &approver.0,
            User::TxOperator(tx_operator) => &tx_operator.0,
            User::Viewer(viewer) => &viewer.0,
        }
    }

    pub fn _is_tx_operator(&self) -> bool {
        matches!(self, User::TxOperator(_))
    }
}

impl std::ops::Deref for User {
    type Target = UserInfo;

    fn deref(&self) -> &Self::Target {
        self.get_info()
    }
}

impl From<Approver> for User {
    fn from(approver: Approver) -> Self {
        User::Approver(approver)
    }
}

impl From<TxOperator> for User {
    fn from(tx_operator: TxOperator) -> Self {
        User::TxOperator(tx_operator)
    }
}

impl From<Admin> for User {
    fn from(admin: Admin) -> Self {
        User::Admin(admin)
    }
}

impl From<Viewer> for User {
    fn from(viewer: Viewer) -> Self {
        User::Viewer(viewer)
    }
}
