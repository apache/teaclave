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

use async_trait::async_trait;
use db_manager::StorageClient;
use proto::TaCommand;
use proto::{GetTeeStatusInput, GetTeeStatusOutput};

use anyhow::Result;
use std::sync::{Arc, RwLock};
use storable::tee_status::OnlineDevice;
use tls_client_processing::TlsClient;

use task_exec::Executable;

pub struct SyncTeeStatusTask {
    db_client: Arc<StorageClient>,
    tls_client: Arc<RwLock<TlsClient>>,
}

impl SyncTeeStatusTask {
    pub fn new(db_client: Arc<StorageClient>, tls_client: Arc<RwLock<TlsClient>>) -> Self {
        Self {
            db_client,
            tls_client,
        }
    }

    fn inner_exec(&self) -> Result<OnlineDevice> {
        let output: GetTeeStatusOutput = self
            .tls_client
            .write()
            .unwrap()
            .invoke(GetTeeStatusInput {}, TaCommand::GetTeeStatus)?;
        let online_device = OnlineDevice::new(output.device_id, output.tee_status);
        Ok(online_device)
    }
}

#[async_trait]
impl Executable for SyncTeeStatusTask {
    async fn exec(&self) {
        match self.inner_exec() {
            Ok(online_device) => {
                self.db_client
                    .put(&online_device)
                    .await
                    .unwrap_or_else(|e| {
                        log::error!("put_tee_status() error: {:?}", e);
                    });
            }
            Err(e) => {
                log::error!("SyncTeeStatusTask::exec() error: {:?}", e);
                self.db_client
                    .delete_entry::<_, OnlineDevice>(&"OnlineDevice".to_string())
                    .await
                    .unwrap_or_else(|e| {
                        log::error!("delete_entry() error: {:?}", e);
                    });
            }
        }
    }
}
