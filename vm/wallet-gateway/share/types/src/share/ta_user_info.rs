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

use crate::share::{CkHash, Role, RoleSet, UserID};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TaUserInfo {
    pubkey_hash: CkHash, // hashmap key
    user_id: UserID,
    roles: RoleSet,
}

impl TaUserInfo {
    pub fn new(pubkey_hash: CkHash, user_id: UserID, roles: RoleSet) -> Self {
        Self {
            pubkey_hash,
            user_id,
            roles,
        }
    }

    pub fn pubkey_hash(&self) -> &CkHash {
        &self.pubkey_hash
    }

    pub fn user_id(&self) -> &UserID {
        &self.user_id
    }

    pub fn roles(&self) -> &RoleSet {
        &self.roles
    }

    pub fn is_admin(&self) -> bool {
        self.roles.contains(&Role::Admin)
    }

    pub fn is_system(&self) -> bool {
        self.roles.contains(&Role::System)
    }

    pub fn is_approver(&self) -> bool {
        self.roles.contains(&Role::Approver)
    }

    pub fn is_tx_operator(&self) -> bool {
        self.roles.contains(&Role::TxOperator)
    }
}
