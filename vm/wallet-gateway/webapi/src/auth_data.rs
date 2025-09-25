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

use anyhow::Result;
use serde::{Deserialize, Serialize};
use types::external::Email;

// a demo AuthData struct, please implement the real one for production
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthData {
    access_token: String,
    email: String,
    id_token: String,
}

impl AuthData {
    // important: for dev, we skip verifying token, for production please implement the verification
    pub async fn get_authorized_email(&self, _auth_url: &url::Url) -> Result<Email> {
        log::warn!("skip verifying token for dev");
        self.email.clone().try_into()
    }
}
