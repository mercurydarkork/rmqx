#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;
extern crate config;
#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate lazy_static;
// #[macro_use]
// extern crate serde;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate futures;

mod codec;
mod options;
mod server;
mod webhook;

use crate::codec::mqtt::*;
use crate::server::*;
use bytes::{Buf, BufMut, Bytes, BytesMut};
use bytestring::ByteString;
use std::fmt::Write;
use tokio::net::TcpStream;
use tokio::time::delay_for;
use tokio::time::Duration;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:1883".to_string());
    let n = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "1".to_string())
        .parse()
        .unwrap();
    let t = std::env::args()
        .nth(3)
        .unwrap_or_else(|| "1".to_string())
        .parse()
        .unwrap();

    for i in 1..=n {
        let addr = addr.to_owned();
        let client_id = format!("{}-{}", i, Uuid::new_v4());
        tokio::spawn(async move {
            Client::connect(
                &addr,
                client_id,
                None,
                Some(ByteString::from("username")),
                Some(Bytes::from("password")),
                t,
            )
            .await;
        });
    }
    // let packet_id = 0u16;
    // let mut paylaod = bytes::BytesMut::from("broadcast ");
    // paylaod.write_str(&packet_id.to_string()).unwrap();
    // client.publish(Publish {
    //     dup: false,
    //     retain: false,
    //     qos: QoS::AtLeastOnce,
    //     topic: ByteString::from("$sys/broadcast"),
    //     packet_id: Some(packet_id),
    //     payload: paylaod.to_bytes(),
    // })?;
    loop {
        use tokio::time::delay_for;
        delay_for(Duration::from_secs(60)).await;
    }
}
