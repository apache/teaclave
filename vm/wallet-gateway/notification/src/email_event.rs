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

use serde::{Deserialize, Serialize};
use storable::pending_tx::PendingTx;
use storable::received_payment::PaymentRecord;
use storable::tx_history::HistoryTx;
use storable::Storable;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NotifyEventInfo {
    pub id: Uuid,
    pub event: NotifyEvent,
}

impl NotifyEventInfo {
    pub fn new(event: NotifyEvent) -> Self {
        Self {
            id: Uuid::new_v4(),
            event,
        }
    }
}

impl Storable<String> for NotifyEventInfo {
    fn unique_id(&self) -> String {
        self.id.to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "event")]
pub enum NotifyEvent {
    HistoryEvent(HistoryTx),
    PendingEvent(PendingTx),
    ReceivedPayment(PaymentRecord),
}
