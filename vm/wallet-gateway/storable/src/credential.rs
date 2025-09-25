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
use serde::{Deserialize, Serialize};
use types::external::Email;
use types::share::{CkPrivateKey, CkPublicKey};

#[derive(Serialize, Deserialize, Debug, Hash)]
pub struct UserCredential {
    email: Email,
    public_key: CkPublicKey,
    private_key: CkPrivateKey,
    cert: Vec<u8>,
}

impl UserCredential {
    pub fn new(
        email: Email,
        public_key: CkPublicKey,
        private_key: CkPrivateKey,
        cert: Vec<u8>,
    ) -> Self {
        Self {
            email,
            public_key,
            private_key,
            cert,
        }
    }
    pub fn _public_key(&self) -> &CkPublicKey {
        &self.public_key
    }
    pub fn private_key(&self) -> &CkPrivateKey {
        &self.private_key
    }
    pub fn cert(&self) -> &Vec<u8> {
        &self.cert
    }
}

impl Storable<Email> for UserCredential {
    fn unique_id(&self) -> Email {
        self.email.clone()
    }
}

impl From<UserCredential> for CkPrivateKey {
    fn from(cred: UserCredential) -> Self {
        cred.private_key
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CaCredential {
    ca_cert: Vec<u8>,
    ca_der: Vec<u8>, //cert.der as trust anchor of verifying server
    ca_key: Vec<u8>,
}

impl CaCredential {
    pub fn new(ca_cert: Vec<u8>, ca_der: Vec<u8>, ca_key: Vec<u8>) -> Self {
        Self {
            ca_cert,
            ca_der,
            ca_key,
        }
    }

    pub fn ca_cert(&self) -> &Vec<u8> {
        &self.ca_cert
    }

    pub fn ca_der(&self) -> &Vec<u8> {
        &self.ca_der
    }

    pub fn ca_key(&self) -> &Vec<u8> {
        &self.ca_key
    }
}

impl Storable<String> for CaCredential {
    fn unique_id(&self) -> String {
        "CaCredential".into()
    }
}
