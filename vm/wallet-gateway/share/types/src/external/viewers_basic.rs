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

use crate::external::Email;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::iter::IntoIterator;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ViewersBasic(HashSet<Email>);

impl ViewersBasic {
    pub fn new(inner: HashSet<Email>) -> Self {
        Self(inner)
    }

    pub fn distinct_viewers(&self) -> HashSet<&Email> {
        self.0.iter().collect()
    }
}

impl IntoIterator for ViewersBasic {
    type Item = Email;
    type IntoIter = std::collections::hash_set::IntoIter<Email>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl std::ops::Deref for ViewersBasic {
    type Target = HashSet<Email>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
