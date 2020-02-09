#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use crate::codec::mqtt::*;
use crate::server::*;
use bytes::Buf;
use bytestring::ByteString;
use coap::{IsMessage, Method, Server};
use futures::{channel::mpsc, SinkExt, StreamExt};
use native_tls::Identity;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
// use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio_util::codec::{Decoder, Encoder, Framed};

pub async fn serve_coap<T: AsRef<str>>(laddr: T) -> Result<()> {
    let mut server = Server::new(laddr.as_ref())?;
    println!("coap listen on {}", laddr.as_ref());
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
pub async fn serve_webapi<T: AsRef<str>>(laddr: T) -> Result<()> {
    use hyper::service::{make_service_fn, service_fn};
    use hyper::{header, Body, Client, Method, Request, Response, Server, StatusCode};
    use std::net::SocketAddr;
    let laddr: SocketAddr = laddr.as_ref().parse().unwrap();
    static INTERNAL_SERVER_ERROR: &[u8] = b"Internal Server Error";
    static NOTFOUND: &[u8] = b"Not Found";

    #[derive(Serialize, Deserialize)]
    struct State {
        clients: usize,
    }

    async fn state_clients() -> Result<Response<Body>> {
        let data = json!({
            "clients":&state.client_num(),
        });
        let res = match serde_json::to_string(&data) {
            Ok(json) => Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json))
                .unwrap(),
            Err(_) => Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(INTERNAL_SERVER_ERROR.into())
                .unwrap(),
        };
        Ok(res)
    }
    async fn handle(req: Request<Body>) -> Result<Response<Body>> {
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/state/_clients") => state_clients().await,
            _ => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(NOTFOUND.into())
                .unwrap()),
        }
    }
    let make_service = make_service_fn(|_conn| async { Ok::<_, Error>(service_fn(handle)) });
    let server = Server::bind(&laddr).serve(make_service);
    println!("webapi listen on {}", laddr);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
    Ok(())
}

pub async fn serve_ws<T: AsRef<str>>(_laddr: T) -> Result<()> {
    Ok(())
}

pub async fn serve_wss<T: AsRef<str>>(_laddr: T, _p12: T, _passwd: T) -> Result<()> {
    Ok(())
}

pub async fn serve_tls<T: AsRef<str>, U: AsRef<std::path::Path>, W: AsRef<str>>(
    laddr: T,
    cert: U,
    passwd: W,
) -> Result<()> {
    let mut listener = TcpListener::bind(laddr.as_ref()).await?;
    let der = fs::read(cert).await?;
    let cert = Identity::from_pkcs12(&der, passwd.as_ref())?;
    let tls_acceptor =
        tokio_tls::TlsAcceptor::from(native_tls::TlsAcceptor::builder(cert).build()?);
    println!("mqtt.tls listen on {}", laddr.as_ref());
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                socket.set_nodelay(true)?;
                socket.set_keepalive(None)?;
                socket.set_recv_buffer_size(4096)?;
                socket.set_send_buffer_size(4096)?;
                let tls_acceptor = tls_acceptor.clone();
                tokio::spawn(async move {
                    if let Ok(socket) = tls_acceptor.accept(socket).await {
                        process(Peer::new(Framed::new(socket, MqttCodec::new())), addr).await
                    }
                });
            }
            Err(e) => println!("error accepting socket; error = {:?}", e),
        }
    }
}

pub async fn serve<T: AsRef<str>>(laddr: T) -> Result<()> {
    use bytes::BufMut;
    use std::fmt::Write;
    use tokio::time::delay_for;
    let mut listener = TcpListener::bind(laddr.as_ref()).await?;
    println!("mqtt listen on {}", laddr.as_ref());
    loop {
        match listener.accept().await {
            Ok((socket, addr)) => {
                socket.set_nodelay(true)?;
                socket.set_keepalive(Some(std::time::Duration::new(60, 0)))?;
                socket.set_ttl(120)?;
                tokio::spawn(async move {
                    process(Peer::new(Framed::new(socket, MqttCodec::new())), addr).await;
                    //println!("addr {} closed", &addr);
                });
            }
            Err(e) => println!("error accepting socket; error = {:?}", e),
        }
    }
}

async fn process<T, U>(peer: Peer<T, U>, addr: SocketAddr)
where
    T: AsyncRead + AsyncWrite + Unpin,
    U: Decoder<Item = Packet, Error = ParseError>,
    U: Encoder<Item = Packet, Error = ParseError>,
{
    let mut peer = peer;
    peer.set_keep_alive(Duration::from_secs(60));
    if let Ok(()) = peer
        .handshake(|conn, tx| -> bool {
            if *&state.exist(&conn.client_id) {
                &state.kick(&conn.client_id);
            }
            &state.add_peer(conn.client_id.clone(), tx);
            true
        })
        .await
    {
        if let Err(e) = peer.process_loop(|_packet| -> bool { true }).await {
            println!(
                "failed to process connection {} {}; error = {}",
                peer.client_id, addr, e
            );
        }
        &state.remove_peer(&peer.client_id);
    }
    drop(peer);
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Message {
    /// 转发Publish消息
    Forward(Publish),

    /// MQTT协议报文
    Mqtt(Packet),
    KeepAlive,
    Kick,
}

pub type Tx = mpsc::UnboundedSender<Message>;

pub type Rx = mpsc::UnboundedReceiver<Message>;

pub struct Shared {
    peers: RwLock<HashMap<ByteString, Tx>>,
}

lazy_static! {
    static ref state: Arc<Shared> = Arc::new(Shared::new());
}

// impl Shared {
//     fn new() -> Self {
//         Shared {
//             peers: RwLock::new(HashMap::new()),
//         }
//     }

//     pub fn client_num(&self) -> usize {
//         self.peers.read().len()
//     }

//     pub fn exist(&self, client_id: &str) -> bool {
//         self.peers.read().contains_key(client_id)
//     }

//     pub fn kick(&self, client_id: &str) {
//         if let Some(tx) = self.peers.write().remove(client_id) {
//             let _ = tx.send(Message::Kick);
//             drop(tx);
//         }
//     }

//     pub fn add_peer(&self, client_id: ByteString, tx: Tx) {
//         self.peers.write().insert(client_id, tx);
//     }

//     pub fn remove_peer(&self, client_id: &str) {
//         if let Some(tx) = self.peers.write().remove(client_id) {
//             drop(tx);
//         }
//     }

//     pub async fn broadcast(&self, publish: Publish) {
//         for peer in self.peers.read().iter() {
//             if let Err(e) = peer.1.send(Message::Forward(publish.clone())) {
//                 println!("broadcast {} {}", peer.0, e);
//             }
//         }
//     }
//     pub async fn forward(&self, client_id: &str, publish: Publish) -> Result<()> {
//         if let Some(tx) = self.peers.read().get(client_id) {
//             if let Err(_e) = tx.send(Message::Forward(publish)) {
//                 return Err(Box::new(ParseError::InvalidClientId));
//             }
//         }
//         Ok(())
//     }

//     pub async fn send(&self, client_id: &str, msg: Message) -> Result<()> {
//         if let Some(tx) = self.peers.read().get(client_id) {
//             if let Err(_e) = tx.send(msg) {
//                 return Err(Box::new(ParseError::InvalidClientId));
//             }
//         }
//         Ok(())
//     }
// }

impl Shared {
    fn new() -> Self {
        Shared {
            peers: RwLock::new(HashMap::new()),
        }
    }

    pub fn client_num(&self) -> usize {
        self.peers.read().len()
    }

    pub fn exist(&self, client_id: &str) -> bool {
        self.peers.read().contains_key(client_id)
    }

    pub fn kick(&self, client_id: &str) {
        if let Some(mut tx) = self.peers.write().remove(client_id) {
            let _ = tx.send(Message::Kick);
            drop(tx);
        }
    }

    pub fn add_peer(&self, client_id: ByteString, tx: Tx) {
        self.peers.write().insert(client_id, tx);
    }

    pub fn remove_peer(&self, client_id: &str) {
        if let Some(tx) = self.peers.write().remove(client_id) {
            drop(tx);
        }
    }

    pub async fn broadcast(&self, publish: Publish) -> Result<()> {
        for peer in self.peers.read().iter() {
            let mut tx = peer.1;
            tx.send(Message::Forward(publish.clone())).await?;
        }
        Ok(())
    }
    pub async fn forward(&self, client_id: &str, publish: Publish) -> Result<()> {
        if let Some(mut tx) = self.peers.read().get(client_id) {
            tx.send(Message::Forward(publish)).await?;
        }
        Ok(())
    }

    pub async fn send(&self, client_id: &str, msg: Message) -> Result<()> {
        if let Some(mut tx) = self.peers.read().get(client_id) {
            tx.send(msg).await?;
        }
        Ok(())
    }
}
