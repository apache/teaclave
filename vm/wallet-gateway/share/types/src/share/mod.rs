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

mod account;
mod address;
mod approval_operations;
mod basic_types;
mod ck_hash;
mod device;
mod network;
mod ta_approval_chain;
mod ta_approval_chain_basic;
mod ta_operators_basic;
mod ta_user_info;
mod tee_config;
mod tee_status;
mod tx;
mod user;
mod wallet;
mod xpub;

// reexport
pub use bitcoin::hex::FromHex;
pub use bitcoin::secp256k1::Secp256k1;

pub use account::{AccountId, MultiChainAccount, MultiChainAccountId, SerializedAccount};

pub use address::btc::{BtcAddress, ClientBtcAddress};
pub use address::eth::EthAddress;

pub use tx::btc::{BtcTransaction, ChangeInfo, RecipientInfo, UtxoInfo};
pub use tx::eth::EthTransaction;
pub use tx::{multichain::MultiChainTransaction, TransactionID, TransactionStatus};

pub use xpub::AccountXpub;

pub use ta_approval_chain::{TaApprovalChain, TaApprovalStage};
pub use ta_approval_chain_basic::{TaApprovalChainBasic, TaApprovalStageBasic};
pub use ta_operators_basic::TaOperatorsBasic;

pub use approval_operations::{ApprovalOperation, ApprovalStatus};

pub use device::DeviceID;
pub use tee_config::{TaWalletInfo, TeeConfig};
pub use tee_status::{ServiceInfo, TeeOnlineStatus};
pub use user::{Role, RoleSet, UserID};
pub use wallet::WalletID;

pub use network::{ChainType, CkNetwork, NetworkType, SUPPORTED_CHAIN_TYPES};

pub use basic_types::*;

pub use ck_hash::{CkHash, CkHasher};

pub use ta_user_info::TaUserInfo;

pub use ethereum_tx_sign::LegacyTransaction;
