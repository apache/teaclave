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
use std::marker::{Send, Sync};

#[async_trait]
pub trait Executable: Send + Sync {
    async fn exec(&self);
}

#[macro_export]
macro_rules! join_periodic_tasks {
    ( $itval:expr, $( $task:expr ),* ) => {
        {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs($itval));
            loop {
                interval.tick().await;
                tokio::join!($($task.exec(),)*);
            }
        }
    };
}

#[macro_export]
macro_rules! join_mixed_frequency_tasks {
    (
        high_freq: $high_interval:expr => [ $( $high_task:expr ),* ],
        low_freq: $low_interval:expr => [ $( $low_task:expr ),* ]
    ) => {
        {
            // Start high frequency tasks
            let high_freq_handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs($high_interval));
                loop {
                    interval.tick().await;
                    tokio::join!($($high_task.exec(),)*);
                }
            });

            // Start low frequency tasks
            let low_freq_handle = tokio::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs($low_interval));
                loop {
                    interval.tick().await;
                    tokio::join!($($low_task.exec(),)*);
                }
            });

            // Wait for both task groups
            tokio::try_join!(high_freq_handle, low_freq_handle)
        }
    };
}
