#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
use crate::codec::mqtt::*;
use crate::server::*;
use bytes::Bytes;
use bytestring::ByteString;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio::time::delay_for;
use tokio::time::{timeout, Duration};
use tokio_util::codec::Framed;

pub struct Client {
    tx: Option<Tx>,
}

impl Client {
    pub async fn connect<A: AsRef<str>>(
        addr: A,
        client_id: String,
        _last_will: Option<LastWill>,
        username: Option<ByteString>,
        password: Option<Bytes>,
    ) -> Result<Self> {
        // let last_will = last_will.unwrap();
        let username = username.unwrap();
        let password = password.unwrap();
        loop {
            match TcpStream::connect(addr.as_ref()).await {
                Ok(socket) => {
                    let paddr = socket.peer_addr().unwrap();
                    if let Ok(stream) = MqttStream::connect(
                        socket,
                        Connect {
                            client_id: ByteString::from(client_id.clone()),
                            last_will: None,
                            clean_session: false,
                            protocol: Protocol::default(),
                            username: Some(username.clone()),
                            password: Some(password.clone()),
                            keep_alive: 60,
                        },
                    )
                    .await
                    {
                        let mut peer = Peer::from(&client_id, stream, paddr);
                        peer.set_keepalive(Duration::from_secs(30));
                        if let Err(e) = peer.evloop(|_packet| -> bool { true }).await {
                            println!(
                                "failed to process connection {}; error = {}",
                                peer.client_id, e
                            );
                        }
                        return Ok(Client { tx: None });
                    }
                }
                Err(e) => println!("tcp.connect {} {}", addr.as_ref(), e),
            }
            delay_for(Duration::from_secs(5)).await;
        }
    }

    pub async fn publish(&self, publish: Publish) -> Result<()> {
        if let Some(tx) = &self.tx {
            let mut tx = tx;
            tx.send(Message::Forward(publish)).await?;
        }
        Ok(())
    }
}
