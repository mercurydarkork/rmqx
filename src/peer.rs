#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]

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
}

pub struct Peer<T, U> {
    pub client_id: ByteString,
    transport: Framed<T, U>,
    ttl: Duration,
    rx: Option<Rx>,
    //state: Arc<Shared>,
}

impl<T, U> Peer<T, U>
where
    T: AsyncRead + AsyncWrite + Unpin,
    U: Decoder<Item = Packet, Error = ParseError>,
    U: Encoder<Item = Packet, Error = ParseError>,
{
    // pub fn new(state: Arc<Shared>, transport: Framed<T, U>) -> Self {
    //     Self {
    //         client_id: bytestring::ByteString::new(),
    //         transport: transport,
    //         ttl: Duration::from_secs(60),
    //         rx: None,
    //         state: state,
    //     }
    // }

    pub fn new(transport: Framed<T, U>) -> Self {
        Self {
            client_id: bytestring::ByteString::new(),
            transport: transport,
            ttl: Duration::from_secs(60),
            rx: None,
        }
    }

    pub fn set_ttl(&mut self, ttl: Duration) {
        self.ttl = ttl
    }

    async fn receive(&mut self) -> Result<Option<Message>> {
        self.receive_timeout(self.ttl).await
    }
    async fn receive_timeout(&mut self, tm: Duration) -> Result<Option<Message>> {
        match timeout(tm, self.next()).await {
            Ok(Some(Ok(msg))) => {
                //println!("recv: {} {:#?}", self.client_id, &msg);
                Ok(Some(msg))
            }
            Ok(Some(Err(e))) => Err(e),
            Ok(None) => Ok(None),
            Err(_) => Err(Box::new(ParseError::Timeout(self.ttl))),
        }
    }

    async fn send(&mut self, packet: Packet) -> Result<()> {
        //println!("peer.send: {} {:#?}", self.client_id, packet);
        if let Err(e) = self.transport.send(packet).await {
            return Err(Box::new(e));
        }
        Ok(())
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

    async fn publish(&mut self, publish: Publish) -> Result<()> {
        self.send(Packet::Publish(publish)).await
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
        self.handshake().await?;
        let (_tx, rx) = mpsc::unbounded_channel();
        self.rx = Some(rx);
        //self.state.addPeer(self.client_id.clone(), tx).await;
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
                Ok(None) => return Ok(()),
                Err(e) => return Err(e),
            }
        }
    }

    pub async fn handshake(&mut self) -> Result<()> {
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
                .await
        } else {
            Err(Box::new(ParseError::Timeout(ttl)))
        }
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
