#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use bytes::{Buf, Bytes};
use bytestring::ByteString;
use futures::Stream;
use futures::{SinkExt, StreamExt};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};
use tokio_util::codec::{Decoder, Encoder, Framed};

use crate::codec::mqtt::*;
use crate::server::*;

#[derive(Debug)]
pub enum Message {
    /// 转发Publish消息
    Forward(Publish),

    /// MQTT协议报文
    Received(Packet),
    KeepAlive,
}

pub struct Peer<T, U> {
    transport: Framed<T, U>,
    pub client_id: ByteString,
    pub keep_alive: Duration,
    pub clean_session: bool,
    pub last_will: Option<LastWill>,
    pub rx: Option<Rx>,
    pub tx: Option<Tx>,
    from: bool,
    // state: Arc<Shared>,
}

impl<T, U> Peer<T, U>
where
    T: AsyncRead + AsyncWrite + Unpin,
    U: Decoder<Item = Packet, Error = ParseError>,
    U: Encoder<Item = Packet, Error = ParseError>,
{
    pub fn new(transport: Framed<T, U>) -> Self {
        Self {
            transport: transport,
            client_id: bytestring::ByteString::new(),
            keep_alive: Duration::from_secs(60),
            clean_session: false,
            last_will: None,
            rx: None,
            tx: None,
            from: false,
        }
    }

    pub fn from(client_id: ByteString, transport: Framed<T, U>) -> Self {
        Self {
            transport: transport,
            client_id: client_id,
            keep_alive: Duration::from_secs(60),
            clean_session: false,
            last_will: None,
            rx: None,
            tx: None,
            from: true,
        }
    }

    // pub async fn connect<F>(addr: &str, f: F) -> tokio::task::JoinHandle<Self>
    // where
    //     F: FnOnce(&mut Peer<T, U>),
    //     F: Send + 'static,
    // {
    // }

    pub fn set_keep_alive(&mut self, keep_alive: Duration) {
        self.keep_alive = keep_alive
    }

    async fn receive(&mut self) -> Result<Option<Message>> {
        self.receive_timeout(self.keep_alive).await
    }
    async fn receive_timeout(&mut self, tm: Duration) -> Result<Option<Message>> {
        match timeout(tm, self.next()).await {
            Ok(Some(Ok(msg))) => {
                //println!("recv: {} {:#?}", self.client_id, &msg);
                Ok(Some(msg))
            }
            Ok(Some(Err(e))) => Err(e),
            Ok(None) => Ok(None),
            Err(_) => {
                if self.from {
                    Ok(Some(Message::KeepAlive))
                } else {
                    Err(Box::new(ParseError::Timeout(self.keep_alive)))
                }
            }
        }
    }

    async fn send(&mut self, packet: Packet) -> Result<()> {
        // println!("peer.send: {} {:#?}", self.client_id, packet);
        if let Err(e) = self.transport.send(packet).await {
            return Err(Box::new(e));
        }
        Ok(())
    }

    async fn send_timeout(&mut self, packet: Packet, tm: Duration) -> Result<()> {
        //println!("peer.send: {} {:#?}", self.client_id, packet);
        if let Err(e) = timeout(tm, self.transport.send(packet)).await {
            return Err(Box::new(e));
        }
        Ok(())
    }

    async fn send_connect(&mut self, connect: Connect, _tm: Duration) -> Result<()> {
        let connect = Packet::Connect(connect);
        self.send(connect).await
    }
    async fn send_disconnect(&mut self) -> Result<()> {
        let disconnect = Packet::Disconnect {};
        self.send(disconnect).await
    }
    async fn send_connect_ack(
        &mut self,
        session_present: bool,
        return_code: ConnectCode,
    ) -> Result<()> {
        let ack = Packet::ConnectAck {
            session_present: session_present,
            return_code: return_code, //版本错误
        };
        self.send(ack).await
    }

    pub async fn publish(&mut self, publish: Publish) -> Result<()> {
        self.send(Packet::Publish(publish)).await
    }

    async fn send_ping(&mut self) -> Result<()> {
        let ping = Packet::PingRequest {};
        self.send(ping).await
    }

    async fn send_pong(&mut self) -> Result<()> {
        let pong = Packet::PingResponse {};
        self.send(pong).await
    }

    async fn send_publish_ack(&mut self, packet_id: u16) -> Result<()> {
        let ack = Packet::PublishAck {
            packet_id: packet_id,
        };
        self.send(ack).await
    }

    async fn send_publish_received(&mut self, packet_id: u16) -> Result<()> {
        let ack = Packet::PublishReceived {
            packet_id: packet_id,
        };
        self.send(ack).await
    }
    async fn send_publish_release(&mut self, packet_id: u16) -> Result<()> {
        let ack = Packet::PublishRelease {
            packet_id: packet_id,
        };
        self.send(ack).await
    }
    async fn send_publish_complete(&mut self, packet_id: u16) -> Result<()> {
        let ack = Packet::PublishComplete {
            packet_id: packet_id,
        };
        self.send(ack).await
    }
    async fn send_subscribe_ack(&mut self, packet_id: u16) -> Result<()> {
        let ack = Packet::SubscribeAck {
            packet_id: packet_id,
            status: vec![SubscribeReturnCode::Success(QoS::AtLeastOnce)],
        };
        self.send(ack).await
    }

    async fn send_unsubscribe_ack(&mut self, packet_id: u16) -> Result<()> {
        let ack = Packet::UnsubscribeAck {
            packet_id: packet_id,
        };
        self.send(ack).await
    }

    pub async fn process(&mut self) -> Result<()> {
        loop {
            match self.receive().await {
                Ok(Some(Message::Forward(publish))) => self.publish(publish).await?,
                Ok(Some(Message::Received(Packet::PingRequest))) => self.send_pong().await?,
                Ok(Some(Message::Received(Packet::Connect(_)))) => {
                    self.send_connect_ack(false, ConnectCode::NotAuthorized)
                        .await?;
                    self.send_disconnect().await?
                }
                Ok(Some(Message::Received(Packet::Publish(publish)))) => {
                    if publish.qos == QoS::AtLeastOnce {
                        self.send_publish_ack(publish.packet_id.unwrap()).await?;
                    } else if publish.qos == QoS::ExactlyOnce {
                        self.send_publish_received(publish.packet_id.unwrap())
                            .await?;
                    }
                }
                Ok(Some(Message::Received(Packet::PublishRelease { packet_id }))) => {
                    self.send_publish_complete(packet_id).await?
                }
                Ok(Some(Message::Received(Packet::PublishReceived { packet_id }))) => {
                    self.send_publish_release(packet_id).await?
                }
                Ok(Some(Message::Received(Packet::Subscribe {
                    packet_id,
                    topic_filters: _,
                }))) => self.send_subscribe_ack(packet_id).await?,
                Ok(Some(Message::Received(Packet::Unsubscribe {
                    packet_id,
                    topic_filters: _,
                }))) => self.send_unsubscribe_ack(packet_id).await?,
                Ok(Some(Message::Received(Packet::PublishAck { .. }))) => {}
                Ok(Some(Message::Received(Packet::Disconnect))) => {}
                Ok(Some(Message::Received(Packet::PublishComplete { .. }))) => {}
                Ok(Some(Message::Received(_))) => {}
                Ok(Some(Message::KeepAlive)) => {
                    self.send_ping().await?;
                }
                Ok(None) => return Ok(()),
                Err(e) => return Err(e),
            }
        }
    }

    pub async fn handshake<F>(&mut self, f: F) -> Result<()>
    where
        F: Fn(&mut Peer<T, U>, Tx),
    {
        let (tx, rx) = mpsc::unbounded_channel();
        self.rx = Some(rx);
        self.tx = Some(tx.clone());
        let ttl = Duration::from_secs(5);
        if let Ok(Some(Message::Received(Packet::Connect(connect)))) =
            self.receive_timeout(ttl).await
        {
            if connect.protocol.level() != 4 {
                self.send_connect_ack(false, ConnectCode::UnacceptableProtocolVersion)
                    .await?;
                return Err(Box::new(ParseError::UnsupportedProtocolLevel));
            }
            self.client_id = connect.client_id;
            self.send_connect_ack(true, ConnectCode::ConnectionAccepted)
                .await?;
            f(self, tx);
            Ok(())
        } else {
            Err(Box::new(ParseError::Timeout(ttl)))
        }
    }

    pub async fn connect<F>(
        &mut self,
        last_will: Option<LastWill>,
        username: Option<ByteString>,
        password: Option<Bytes>,
        mut f: F,
    ) -> Result<()>
    where
        F: FnMut(&mut Peer<T, U>, Tx),
    {
        let connect = Connect {
            client_id: self.client_id.clone(),
            protocol: Protocol::default(),
            clean_session: self.clean_session,
            keep_alive: self.keep_alive.as_secs() as u16,
            last_will: last_will,
            username: username,
            password: password,
        };
        let ttl = Duration::from_secs(5);
        self.send_connect(connect, ttl).await?;
        match self.receive_timeout(ttl).await {
            Ok(Some(Message::Received(Packet::ConnectAck {
                session_present: _,
                return_code,
            }))) => {
                if return_code == ConnectCode::ConnectionAccepted {
                    let (tx, rx) = mpsc::unbounded_channel();
                    self.rx = Some(rx);
                    f(self, tx);
                }
                Ok(())
            }
            Ok(ret) => {
                println!("{:#?}", ret);
                Ok(())
            }
            Err(e) => {
                println!("{}", e);
                Err(e)
            }
        }
        // if let Ok(Some(Message::Received(Packet::ConnectAck {
        //     session_present: _,
        //     return_code,
        // }))) = self.receive_timeout(ttl).await
        // {
        //     if return_code == ConnectCode::ConnectionAccepted {
        //         let (tx, rx) = mpsc::unbounded_channel();
        //         self.rx = Some(rx);
        //         f(self, tx);
        //     }
        //     Ok(())
        // } else {
        //     Err(Box::new(ParseError::Timeout(ttl)))
        // }
    }
}

impl<T, U> Stream for Peer<T, U>
where
    T: AsyncRead + Unpin,
    U: Decoder<Item = Packet, Error = ParseError>,
{
    type Item = Result<Message>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(rx) = &mut self.rx {
            if let Poll::Ready(Some(v)) = Pin::new(rx).poll_next(cx) {
                return Poll::Ready(Some(Ok(Message::Forward(v))));
            }
        }
        let result: Option<_> = futures::ready!(Pin::new(&mut self.transport).poll_next(cx));
        Poll::Ready(match result {
            Some(Ok(msg)) => Some(Ok(Message::Received(msg))),
            Some(Err(err)) => Some(Err(Box::new(err))),
            None => None,
        })
    }
}
