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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TxSubmissionResult {
    Accepted(NetworkTxHash),
    Rejected(NetworkErrMsg),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkTxHash(String);

impl std::convert::From<String> for NetworkTxHash {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl std::ops::Deref for NetworkTxHash {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkErrMsg(String);

impl std::convert::From<String> for NetworkErrMsg {
    fn from(msg: String) -> Self {
        Self(msg)
    }
}

impl std::convert::From<std::io::Error> for NetworkErrMsg {
    fn from(e: std::io::Error) -> Self {
        Self(e.to_string())
    }
}
