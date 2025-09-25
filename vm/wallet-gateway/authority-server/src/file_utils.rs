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

use anyhow::Result;
use std::path::Path;
use std::time::SystemTime;
use url::Url;

pub enum ExportedFileType {
    Cert,
    WalletInfo,
    SignedBackupList,
}
impl ExportedFileType {
    fn as_str(&self) -> &str {
        match self {
            Self::Cert => "cert",
            Self::WalletInfo => "wallet_info",
            Self::SignedBackupList => "signed_backup_list",
        }
    }
}

pub fn export_file(
    export_base_path: &Path,
    export_base_url: &Url,
    id: impl Into<String>,
    file_type: &ExportedFileType,
    content: &[u8],
    // if the file (serialized struct) has original timestamp, use it as filename
    // otherwise use current timestamp
    timestamp: Option<u64>,
) -> Result<Url> {
    let tms = match timestamp {
        Some(t) => t,
        None => SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs(),
    };
    let file_name_prefix = format!("{}.{}.", Into::<String>::into(id), file_type.as_str(),);
    // file name: $ID.$TYPE.$TIMESTAMP
    let file_name_with_timestamp = format!("{}{}", file_name_prefix, tms);
    std::fs::write(export_base_path.join(&file_name_with_timestamp), content)?;
    // create symbolic link named as "$ID.$TYPE.latest"
    let latest_file_name = format!("{}latest", file_name_prefix);
    // remove symlink if it existd
    if std::path::Path::new(&export_base_path.join(&latest_file_name)).exists() {
        std::fs::remove_file(export_base_path.join(&latest_file_name))?;
    }
    std::os::unix::fs::symlink(
        &file_name_with_timestamp,
        export_base_path.join(&latest_file_name),
    )?;
    log::info!("symbolic link created: {}", latest_file_name);

    let url = export_base_url.join(&latest_file_name)?;
    Ok(url)
}
