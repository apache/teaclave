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

use anyhow::{bail, ensure, Result};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use types::external::Email;

const ADMIN_EMAIL: [&str; 1] = ["admin@ck.com"];

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub enum ConsoleUserRole {
    Admin,
    DeviceOperator,
}

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub enum ConsoleUser {
    Admin(Admin),
    DeviceOperator(DeviceOperator),
}
impl ConsoleUser {
    pub fn _get_info(&self) -> &ConsoleUserInfo {
        match self {
            ConsoleUser::Admin(admin) => &admin.0,
            ConsoleUser::DeviceOperator(device_operator) => &device_operator.0,
        }
    }
    pub fn get_role(&self) -> ConsoleUserRole {
        match self {
            ConsoleUser::Admin(_) => ConsoleUserRole::Admin,
            ConsoleUser::DeviceOperator(_) => ConsoleUserRole::DeviceOperator,
        }
    }
}
impl TryFrom<ConsoleUserInfo> for ConsoleUser {
    type Error = anyhow::Error;

    fn try_from(user_info: ConsoleUserInfo) -> Result<Self> {
        if user_info.is_admin() {
            Ok(ConsoleUser::Admin(Admin(user_info)))
        } else if user_info.is_device_operator() {
            Ok(ConsoleUser::DeviceOperator(DeviceOperator(user_info)))
        } else {
            bail!("unknown role")
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct ConsoleUserInfo {
    pub email: Email,
}
impl ConsoleUserInfo {
    pub fn new(email: Email) -> Self {
        Self { email }
    }
    pub fn is_admin(&self) -> bool {
        if ADMIN_EMAIL.contains(&self.email.0.as_str()) {
            return true;
        }
        false
    }
    pub fn is_device_operator(&self) -> bool {
        false
    }
}

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct Admin(pub ConsoleUserInfo);

#[derive(Serialize, Deserialize, Debug, Hash, Eq, PartialEq)]
pub struct DeviceOperator(pub ConsoleUserInfo);

impl TryFrom<ConsoleUserInfo> for Admin {
    type Error = anyhow::Error;

    fn try_from(user_info: ConsoleUserInfo) -> Result<Self> {
        ensure!(user_info.is_admin(), "not admin");
        let admin = Admin(user_info);
        Ok(admin)
    }
}

impl TryFrom<ConsoleUserInfo> for DeviceOperator {
    type Error = anyhow::Error;

    fn try_from(user_info: ConsoleUserInfo) -> Result<Self> {
        ensure!(user_info.is_device_operator(), "not device operator");
        let device_operator = DeviceOperator(user_info);
        Ok(device_operator)
    }
}
