#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use crate::codec::mqtt::*;
use crate::options;
use crate::server::*;
use bytes::Buf;
use bytes::Bytes;
use bytestring::ByteString;
use chrono::prelude::Local;
use chrono::{NaiveDate, NaiveDateTime};
use reqwest::{Client, ClientBuilder};
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WebhookError {
    #[error("on connect error")]
    OnConnectError,
    #[error("on subscribe error")]
    OnSubscribeError,
}

pub struct Webhook {
    on_connect_url: Option<String>,
    on_disconnect_url: Option<String>,
    on_subscribe_url: Option<String>,
    on_unsubscribe_url: Option<String>,
    on_publish_url: Option<String>,
    on_delivered_url: Option<String>,
    on_deliver_timeout_url: Option<String>,
    on_kick_url: Option<String>,
    client: Client,
}

impl Webhook {
    pub fn new(hook: options::Webhook) -> Self {
        Self {
            on_connect_url: hook.on_connect,
            on_disconnect_url: hook.on_disconnect,
            on_subscribe_url: hook.on_subscribe,
            on_unsubscribe_url: hook.on_unsubscribe,
            on_publish_url: hook.on_publish,
            on_delivered_url: hook.on_delivered,
            on_deliver_timeout_url: hook.on_deliver_timeout,
            on_kick_url: hook.on_kick,
            client: Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap(),
        }
    }

    pub async fn on_connect(
        &self,
        client_id: ByteString,
        username: ByteString,
        password: Bytes,
    ) -> Result<()> {
        if let Some(url) = &self.on_connect_url {
            let body = serde_json::json!({
               "client_id":client_id.to_string(),
               "username":username.to_string(),
               "password":password.bytes(),
               "timestamp":Local::now().timestamp_millis(),
            });
            let res = &self.client.post(url).json(&body).send().await?;
            if res.status() != 200 {
                return Err(Box::new(WebhookError::OnConnectError));
            }
            Ok(())
        } else {
            Err(Box::new(WebhookError::OnConnectError))
        }
    }

    pub async fn on_disconnect(&self, client_id: ByteString) -> Result<()> {
        if let Some(url) = &self.on_disconnect_url {
            let body = serde_json::json!({
               "client_id":client_id.to_string(),
               "timestamp":Local::now().timestamp_millis(),
            });
            &self.client.post(url).json(&body).send().await?;
        }
        Ok(())
    }

    pub async fn on_kick(&self, client_id: ByteString) -> Result<()> {
        if let Some(url) = &self.on_kick_url {
            let body = serde_json::json!({
               "client_id":client_id.to_string(),
               "timestamp":Local::now().timestamp_millis(),
            });
            &self.client.post(url).json(&body).send().await?;
        }
        Ok(())
    }

    pub async fn on_subscribe(
        &self,
        client_id: ByteString,
        _topics: Vec<(ByteString, u8)>,
    ) -> Result<()> {
        if let Some(url) = &self.on_subscribe_url {
            let body = serde_json::json!({
               "client_id":client_id.to_string(),
               //"topics":topics,
               "timestamp":Local::now().timestamp_millis(),
            });
            let res = &self.client.post(url).json(&body).send().await?;
            if res.status() != 200 {
                return Err(Box::new(WebhookError::OnSubscribeError));
            }
            Ok(())
        } else {
            Err(Box::new(WebhookError::OnSubscribeError))
        }
    }
    pub async fn on_unsubscribe(&self, client_id: ByteString, topic: ByteString) -> Result<()> {
        if let Some(url) = &self.on_unsubscribe_url {
            let body = serde_json::json!({
               "client_id":client_id.to_string(),
               "topic":topic.to_string(),
               "timestamp":Local::now().timestamp_millis(),
            });
            &self.client.post(url).json(&body).send().await?;
        }
        Ok(())
    }
    pub async fn on_publish(&self, client_id: ByteString, publish: Publish) -> Result<()> {
        if let Some(url) = &self.on_publish_url {
            let body = serde_json::json!({
               "client_id":client_id.to_string(),
               "dup":publish.dup,
               "retain":publish.retain,
               "qos":publish.qos as u8,
               "packet_id":publish.packet_id,
               "topic":publish.topic.to_string(),
               "payload":publish.payload.bytes(),
               "timestamp":Local::now().timestamp_millis(),
            });
            &self.client.post(url).json(&body).send().await?;
        }
        Ok(())
    }
    pub async fn on_delivered(
        &self,
        client_id: ByteString,
        publish: Publish,
        duration: Duration,
    ) -> Result<()> {
        if let Some(url) = &self.on_publish_url {
            let body = serde_json::json!({
               "client_id":client_id.to_string(),
               "dup":publish.dup,
               "retain":publish.retain,
               "qos":publish.qos as u8,
               "packet_id":publish.packet_id,
               "topic":publish.topic.to_string(),
               "payload":publish.payload.bytes(),
               "duration":duration.as_secs(),
               "timestamp":Local::now().timestamp_millis(),
            });
            &self.client.post(url).json(&body).send().await?;
        }
        Ok(())
    }
    pub async fn on_deliver_timeout(
        &self,
        client_id: ByteString,
        publish: Publish,
        duration: Duration,
    ) -> Result<()> {
        if let Some(url) = &self.on_publish_url {
            let body = serde_json::json!({
               "client_id":client_id.to_string(),
               "dup":publish.dup,
               "retain":publish.retain,
               "qos":publish.qos as u8,
               "packet_id":publish.packet_id,
               "topic":publish.topic.to_string(),
               "payload":publish.payload.bytes(),
               "duration":duration.as_secs(),
               "timestamp":Local::now().timestamp_millis(),
            });
            &self.client.post(url).json(&body).send().await?;
        }
        Ok(())
    }
}
