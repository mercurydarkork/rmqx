#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]

use crate::codec::mqtt::*;
use bytes::Buf;
use bytestring::ByteString;
use coap::{IsMessage, Method, Server};
use futures::{SinkExt, StreamExt};
// use native_tls::Identity;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::Duration;
use tokio_util::codec::Framed;

pub async fn serve_coap<T: AsRef<str>>(laddr: T) -> Result<()> {
    let mut server = Server::new(laddr.as_ref())?;
    println!("listening coap.udp on {}", laddr.as_ref());
    server
        .run(move |request| {
            match request.get_method() {
                &Method::Get => println!("request by get {}", request.get_path()),
                &Method::Post => println!(
                    "request by post {}",
                    String::from_utf8(request.message.payload).unwrap()
                ),
                &Method::Put => println!(
                    "request by put {}",
                    String::from_utf8(request.message.payload).unwrap()
                ),
                _ => println!("request by other method"),
            };
            return match request.response {
                Some(mut message) => {
                    message.set_payload(b"OK".to_vec());
                    Some(message)
                }
                _ => None,
            };
        })
        .await?;
    Ok(())
}
pub async fn serve_webapi<T: AsRef<str>>(_laddr: T) -> Result<()> {
    Ok(())
}

// pub async fn serve_ws<T: AsRef<str>>(_laddr: T) -> Result<()> {
//     Ok(())
// }

// pub async fn serve_wss<T: AsRef<str>>(_laddr: T) -> Result<()> {
//     Ok(())
// }

// pub async fn serve_tls<T: AsRef<str>, U: AsRef<std::path::Path>, W: AsRef<str>>(
//     laddr: T,
//     cert: U,
//     passwd: W,
// ) -> Result<()> {
//     let mut listener = TcpListener::bind(laddr.as_ref()).await?;
//     let der = fs::read(cert).await?;
//     let cert = Identity::from_pkcs12(&der, passwd.as_ref())?;
//     let tls_acceptor =
//         tokio_tls::TlsAcceptor::from(native_tls::TlsAcceptor::builder(cert).build()?);
//     println!("listening mqtt.tls on {}", laddr.as_ref());
//     loop {
//         match listener.accept().await {
//             Ok((socket, addr)) => {
//                 socket.set_nodelay(true)?;
//                 socket.set_keepalive(None)?;
//                 socket.set_recv_buffer_size(4096)?;
//                 socket.set_send_buffer_size(4096)?;
//                 let tls_acceptor = tls_acceptor.clone();
//                 tokio::spawn(async move {
//                     if let Ok(socket) = tls_acceptor.accept(socket).await {
//                         let mut peer = crate::peer::Peer::new(
//                             Arc::clone(&state),
//                             Framed::new(socket, MqttCodec::new()),
//                         );
//                         peer.set_ttl(Duration::from_secs(15));
//                         if let Err(e) = peer.process().await {
//                             println!("failed to process connection {}; error = {}", addr, e);
//                         }
//                         println!("connection {} closed", addr);
//                     }
//                 });
//             }
//             Err(e) => println!("error accepting socket; error = {:?}", e),
//         }
//     }
// }

lazy_static! {
    static ref state: Arc<Shared> = Arc::new(Shared::new());
}
pub async fn serve<T: AsRef<str>>(laddr: T) -> Result<()> {
    use bytes::BufMut;
    use std::fmt::Write;
    use tokio::time::delay_for;
    tokio::spawn(async move {
        let mut packet_id = 0u16;
        let stat = Arc::clone(&state);
        loop {
            let mut paylaod = bytes::BytesMut::from("broadcast ");
            paylaod.write_str(&packet_id.to_string()).unwrap();
            let _res = stat
                .broadcast(Publish {
                    dup: false,
                    retain: false,
                    qos: QoS::AtLeastOnce,
                    topic: ByteString::from("$sys/broadcast"),
                    packet_id: Some(packet_id),
                    payload: paylaod.to_bytes(),
                })
                .await;
            if u16::max_value() == packet_id {
                packet_id = 0;
            } else {
                packet_id += 1;
            }
            delay_for(Duration::from_millis(10000)).await;
        }
    });
    let mut listener = TcpListener::bind(laddr.as_ref()).await?;
    println!("listening mqtt.tcp on {}", laddr.as_ref());
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                socket.set_nodelay(true)?;
                socket.set_keepalive(None)?;
                socket.set_recv_buffer_size(4096)?;
                socket.set_send_buffer_size(4096)?;
                tokio::spawn(async move {
                    let stat = Arc::clone(&state);
                    let mut peer =
                        crate::peer::Peer::new(stat, Framed::new(socket, MqttCodec::new()));
                    peer.set_ttl(Duration::from_secs(15));
                    if let Err(e) = peer.process().await {
                        println!(
                            "failed to process connection {} {}; error = {}",
                            peer.client_id, addr, e
                        );
                    }
                    //println!("connection {} closed", addr);
                    &state.removePeer(&peer.client_id).await;
                });
            }
            Err(e) => println!("error accepting socket; error = {:?}", e),
        }
    }
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub type Tx = mpsc::UnboundedSender<Publish>;

pub type Rx = mpsc::UnboundedReceiver<Publish>;

pub struct Shared {
    peers: RwLock<HashMap<ByteString, Tx>>,
    num: RwLock<usize>,
}

impl Shared {
    fn new() -> Self {
        Shared {
            peers: RwLock::new(HashMap::new()),
            num: RwLock::new(0),
        }
    }

    pub async fn client_num(&self) -> usize {
        self.peers.read().await.len()
    }
    pub async fn addPeer(&self, client_id: ByteString, tx: Tx) {
        self.peers.write().await.insert(client_id.clone(), tx);
        // println!("add peer {} {}", client_id, self.peers.read().await.len());
    }
    pub async fn removePeer(&self, client_id: &ByteString) -> Option<Tx> {
        let ret = self.peers.write().await.remove(client_id);
        // println!(
        //     "remove peer {} {}",
        //     client_id,
        //     self.peers.read().await.len()
        // );
        ret
    }
    pub async fn broadcast(&self, message: Publish) {
        // if let Ok(mut peers) = self.peers.try_lock() {
        //     for peer in peers.iter_mut() {
        //         match peer.1.send(message.clone()) {
        //             Ok(_) => return Ok(()),
        //             Err(_) => return Err(Box::new(ParseError::InvalidClientId)),
        //         }
        //     }
        // } else {
        //     println!("获取锁失败");
        // }
        println!("broadcast {}", self.client_num().await);
        for peer in self.peers.read().await.iter() {
            if let Err(e) = peer.1.send(message.clone()) {
                println!("broadcast {} {}", peer.0, e);
            }
        }
    }
    pub async fn forward(&self, client_id: ByteString, message: Publish) -> Result<()> {
        // if let Ok(peers) = self.peers.try_lock() {
        //     if let Some(tx) = peers.get(&client_id) {
        //         match tx.send(message.clone()) {
        //             Ok(_) => return Ok(()),
        //             Err(_) => return Err(Box::new(ParseError::InvalidClientId)),
        //         }
        //     }
        // } else {
        //     println!("获取锁失败");
        // }
        if let Some(tx) = self.peers.read().await.get(&client_id) {
            if let Err(_e) = tx.send(message.clone()) {
                return Err(Box::new(ParseError::InvalidClientId));
            }
        }
        Ok(())
    }
}
