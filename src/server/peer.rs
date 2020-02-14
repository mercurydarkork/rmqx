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
//use tokio::sync::mpsc;
use crate::codec::mqtt::*;
use crate::server::*;
use futures::channel::mpsc;
use std::net::SocketAddr;
use tokio::time::{timeout, Duration};
use tokio_util::codec::{Decoder, Encoder, Framed};

pub struct Peer<T> {
    pub remote_addr: SocketAddr,
    pub client_id: ByteString,
    pub keep_alive: Duration,
    pub clean_session: bool,
    pub last_will: Option<LastWill>,
    pub username: Option<ByteString>,
    pub password: Option<Bytes>,
    pub rx: Rx,
    pub tx: Tx,
    stream: MqttStream<T>,
    from: bool,
    // state: Arc<Shared>,
}

impl<T> Peer<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(stream: MqttStream<T>, addr: SocketAddr) -> Self {
        let (tx, rx) = mpsc::unbounded();
        Self {
            remote_addr: addr,
            stream: stream,
            client_id: ByteString::new(),
            keep_alive: Duration::from_secs(60),
            clean_session: false,
            last_will: None,
            username: None,
            password: None,
            rx: rx,
            tx: tx,
            from: false,
        }
    }

    pub fn from(client_id: &str, stream: MqttStream<T>, addr: SocketAddr) -> Self {
        let (tx, rx) = mpsc::unbounded();
        Self {
            remote_addr: addr,
            stream: stream,
            client_id: ByteString::from(client_id),
            keep_alive: Duration::from_secs(60),
            clean_session: false,
            last_will: None,
            username: None,
            password: None,
            rx: rx,
            tx: tx,
            from: true,
        }
    }

    pub fn set_keepalive(&mut self, keep_alive: Duration) {
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
            Err(_) => Ok(Some(Message::Timeout(tm))),
        }
    }

    async fn send(&mut self, packet: Packet) -> Result<()> {
        //println!("peer.send: {} {:#?}", self.client_id, packet);
        if let Err(e) = self.stream.send(packet).await {
            return Err(Box::new(e));
        }
        Ok(())
    }

    async fn send_timeout(&mut self, packet: Packet, tm: Duration) -> Result<()> {
        //println!("peer.send: {} {:#?}", self.client_id, packet);
        if let Err(e) = timeout(tm, self.stream.send(packet)).await {
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

    pub async fn evloop<F>(&mut self, f: F) -> Result<()>
    where
        F: Fn(&Packet) -> bool,
    {
        while let Ok(Some(msg)) = self.receive().await {
            match msg {
                Message::Timeout(..) => {
                    if self.from {
                        self.send_ping().await?;
                    } else {
                        return Ok(());
                    }
                }
                Message::Kick => return Ok(()), //被踢掉或者连接断开了
                Message::Forward(publish) => self.publish(publish).await?,
                Message::Mqtt(packet) => {
                    match packet {
                        Packet::Publish(publish) => {
                            if publish.qos == QoS::AtLeastOnce {
                                self.send_publish_ack(publish.packet_id.unwrap()).await?;
                                f(&Packet::Publish(publish));
                            } else if publish.qos == QoS::ExactlyOnce {
                                self.send_publish_received(publish.packet_id.unwrap())
                                    .await?;
                            }
                        }
                        Packet::PublishRelease { packet_id } => {
                            self.send_publish_complete(packet_id).await?;
                            f(&Packet::PublishRelease { packet_id });
                        }
                        Packet::PublishReceived { packet_id } => {
                            self.send_publish_release(packet_id).await?;
                        }
                        Packet::PublishAck { packet_id } => {
                            f(&Packet::PublishAck { packet_id });
                        }
                        Packet::Disconnect => {
                            f(&Packet::Disconnect);
                        }
                        Packet::PublishComplete { packet_id } => {
                            f(&Packet::PublishComplete { packet_id });
                        }
                        Packet::Subscribe {
                            packet_id,
                            topic_filters,
                        } => {
                            if f(&Packet::Subscribe {
                                packet_id,
                                topic_filters,
                            }) {
                                self.send_subscribe_ack(packet_id).await?;
                            }
                            //todo 订阅失败处理
                        }
                        Packet::Unsubscribe {
                            packet_id,
                            topic_filters,
                        } => {
                            self.send_unsubscribe_ack(packet_id).await?;
                            f(&Packet::Unsubscribe {
                                packet_id,
                                topic_filters,
                            });
                        }
                        Packet::PingRequest => self.send_pong().await?,
                        Packet::Connect(_) => {
                            self.send_connect_ack(false, ConnectCode::NotAuthorized)
                                .await?;
                            self.send_disconnect().await?;
                            //todo 处理异常数据包
                            return Ok(());
                        }
                        packet => {
                            println!("mqtt packet {:#?}", packet);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    // pub async fn handshake<F>(&mut self, f: F) -> Result<()>
    // where
    //     F: Fn(&Connect, Tx) -> bool,
    // {
    //     let ttl = Duration::from_secs(5);
    //     if let Ok(Some(Message::Mqtt(Packet::Connect(connect)))) = self.receive_timeout(ttl).await {
    //         if connect.protocol.level() != 4 {
    //             self.send_connect_ack(false, ConnectCode::UnacceptableProtocolVersion)
    //                 .await?;
    //             return Err(Box::new(MqttError::UnsupportedProtocolLevel));
    //         }
    //         //let (tx, rx) = mpsc::unbounded_channel();
    //         let (tx, rx) = mpsc::unbounded();
    //         if f(&connect, tx.clone()) {
    //             self.client_id = connect.client_id;
    //             self.rx = Some(rx);
    //             self.tx = Some(tx);
    //             self.send_connect_ack(true, ConnectCode::ConnectionAccepted)
    //                 .await?;
    //         }
    //         Ok(())
    //     } else {
    //         Err(Box::new(MqttError::Timeout(ttl)))
    //     }
    // }

    // pub async fn connect<F>(
    //     &mut self,
    //     last_will: Option<LastWill>,
    //     username: Option<ByteString>,
    //     password: Option<Bytes>,
    //     mut f: F,
    // ) -> Result<()>
    // where
    //     F: FnMut(&mut Peer<T, U>, Tx),
    // {
    //     let connect = Connect {
    //         client_id: ByteString::from(self.client_id.to_string()),
    //         protocol: Protocol::default(),
    //         clean_session: self.clean_session,
    //         keep_alive: self.keep_alive.as_secs() as u16,
    //         last_will: last_will,
    //         username: username,
    //         password: password,
    //     };
    //     let ttl = Duration::from_secs(5);
    //     self.send_connect(connect, ttl).await?;
    //     match self.receive_timeout(ttl).await {
    //         Ok(Some(Message::Mqtt(Packet::ConnectAck {
    //             session_present: _,
    //             return_code,
    //         }))) => {
    //             if return_code == ConnectCode::ConnectionAccepted {
    //                 //let (tx, rx) = mpsc::unbounded_channel();
    //                 let (tx, rx) = mpsc::unbounded();
    //                 self.rx = Some(rx);
    //                 f(self, tx);
    //             }
    //             Ok(())
    //         }
    //         Ok(ret) => {
    //             println!("{:#?}", ret);
    //             Ok(())
    //         }
    //         Err(e) => {
    //             println!("{}", e);
    //             Err(e)
    //         }
    //     }
    // }
}

impl<T> Stream for Peer<T>
where
    T: AsyncRead + Unpin,
{
    type Item = Result<Message>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // if let Some(rx) = &mut self.rx {
        // }
        if let Poll::Ready(Some(msg)) = Pin::new(&mut self.rx).poll_next(cx) {
            return Poll::Ready(Some(Ok(msg)));
        }
        let result: Option<_> = ready!(Pin::new(&mut self.stream).poll_next(cx));
        Poll::Ready(match result {
            Some(Ok(msg)) => Some(Ok(Message::Mqtt(msg))),
            Some(Err(err)) => Some(Err(Box::new(err))),
            None => None,
        })
    }
}

// impl<T> Drop for Peer<T> {
//     fn drop(&mut self) {
//         drop(&mut self.client_id);
//         drop(&mut self.stream);
//         // drop(&mut self.rx);
//         // drop(&mut self.tx);
//     }
// }
