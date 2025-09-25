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

mod device;
mod transaction;
mod wallet;

pub use device::*;
pub use transaction::*;
pub use wallet::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct TlsCommandRequest {
    pub command: TaCommand,
    pub request: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum TaCommand {
    // tls cmds
    NewTlsSession,
    CloseTlsSession,
    DoTlsRead,
    DoTlsWrite,
    StartServer,
    // wallet cmds
    SyncWithTee,
    AddAccount,
    SyncFromTeeWallet, // deprecated
    // transaction cmds
    CreateTransaction,
    ApproveTransaction,
    RecallTransaction,
    SignTransaction,
    ListPendingTransaction,
    // backup cmds
    InitBoard,
    BackupWallet,
    RestoreWallet,
    ClearWalletStorage,
    // tee status cmds
    GetTeeStatus,
    // unknown command
    Unknown,
}

impl From<u32> for TaCommand {
    #[inline]
    fn from(value: u32) -> TaCommand {
        match value {
            0 => TaCommand::NewTlsSession,
            1 => TaCommand::CloseTlsSession,
            2 => TaCommand::DoTlsRead,
            3 => TaCommand::DoTlsWrite,
            4 => TaCommand::StartServer,
            5 => TaCommand::SyncWithTee,
            6 => TaCommand::AddAccount,
            7 => TaCommand::SyncFromTeeWallet,
            8 => TaCommand::CreateTransaction,
            9 => TaCommand::ApproveTransaction,
            10 => TaCommand::RecallTransaction,
            11 => TaCommand::SignTransaction,
            12 => TaCommand::ListPendingTransaction,
            13 => TaCommand::InitBoard,
            14 => TaCommand::BackupWallet,
            15 => TaCommand::RestoreWallet,
            16 => TaCommand::ClearWalletStorage,
            17 => TaCommand::GetTeeStatus,
            _ => TaCommand::Unknown,
        }
    }
}
