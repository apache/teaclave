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

use crate::share::{AccountXpub, EthAddress};
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EthAccount {
    xpub: AccountXpub,
}

impl EthAccount {
    pub fn new(xpub: AccountXpub) -> Self {
        Self { xpub }
    }

    pub fn eth_address(&self) -> EthAddress {
        let inner = self.id().take();
        EthAddress::new(inner)
    }

    pub fn invoice_address(&self) -> Result<EthAddress> {
        Ok(self.eth_address())
    }

    pub fn id(&self) -> crate::share::AccountId {
        self.xpub.compute_id()
    }
}
