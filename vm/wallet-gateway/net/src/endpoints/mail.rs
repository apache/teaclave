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

use anyhow::{anyhow, Result};
use notification::email_task::SendEmailTask;

pub struct MailServiceEndpoint {
    url: url::Url,
    domain: String,
    token: String,
    client: reqwest::Client,
}

impl MailServiceEndpoint {
    pub fn new(domain: String, token: String) -> Self {
        let url = url::Url::parse("https://api.mailgun.net/v3").unwrap();
        Self {
            url,
            domain,
            token,
            client: reqwest::Client::new(),
        }
    }
    pub async fn send_email(&self, send_email_tasks: &[SendEmailTask]) -> Result<()> {
        let client = reqwest::Client::builder().build()?;
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("Authorization", self.token.parse()?);
        let url = url::Url::parse(&format!(
            "https://api.mailgun.net/v3/{}/messages",
            self.domain
        ))?;

        let from = format!("CK <notification@{}>", self.domain);

        for send_email_task in send_email_tasks.iter() {
            let email = &send_email_task.email;
            let template_name = send_email_task.template_name.clone();
            let email_template = send_email_task.email_template.clone();
            let to: String = email.clone().into();

            let form = reqwest::multipart::Form::new()
                .text("from", from.clone())
                .text("to", to.clone())
                .text("template", template_name)
                .text("h:X-Mailgun-Variables", email_template);
            let response = client
                .request(reqwest::Method::POST, url.clone())
                .headers(headers.clone())
                .multipart(form)
                .send()
                .await?;

            if !response.status().is_success() {
                let body = response.text().await?;
                log::error!("send_email response body: {:?}", body);
                return Err(anyhow!("send_email failed"));
            }
            log::info!("sent email to {:?}", send_email_task.email);
        }
        Ok(())
    }
}
