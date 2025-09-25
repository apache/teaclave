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

mod tee_device_info;
mod tee_wallet;
mod tx;
mod wallet_secret;
mod xpriv;

pub use tx::btc::{CkChangeInfo, CkRecipientInfo, CkUtxoInfo};

pub use xpriv::btc::BtcAccountXpriv;
pub use xpriv::eth::EthAccountXpriv;
pub use xpriv::master::{AccountXpriv, MasterXpriv};

pub use tee_device_info::TeeDeviceInfo;
pub use tee_wallet::{TeeWallet, TeeWalletForBackup};
pub use tx::tee_tx::TeeTransaction;
pub use wallet_secret::WalletSecret;

#[cfg(feature = "testnet")]
pub const BTC_NETWORK: bitcoin::Network = bitcoin::Network::Testnet;
#[cfg(not(feature = "testnet"))]
pub const BTC_NETWORK: bitcoin::Network = bitcoin::Network::Bitcoin;
