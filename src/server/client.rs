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
        last_will: Option<LastWill>,
        username: Option<ByteString>,
        password: Option<Bytes>,
    ) -> Result<Self> {
        loop {
            match TcpStream::connect(addr.as_ref()).await {
                Ok(socket) => {
                    let paddr = socket.peer_addr().unwrap();
                    let codec = Framed::new(socket, MqttCodec::new(paddr));
                    let mut peer = Peer::from(&client_id, codec);
                    peer.set_keepalive(Duration::from_secs(30));
                    let mut c = Client { tx: None };
                    match peer
                        .connect(
                            last_will.clone(),
                            username.clone(),
                            password.clone(),
                            |_p, tx| c.tx = Some(tx),
                        )
                        .await
                    {
                        Ok(()) => {
                            tokio::spawn(async move {
                                if let Err(e) = peer.process_loop(|_packet| -> bool { true }).await
                                {
                                    println!(
                                        "failed to process connection {}; error = {}",
                                        peer.client_id, e
                                    );
                                }
                            });
                            return Ok(c);
                        }
                        Err(e) => println!("peer.connect {} {}", addr.as_ref(), e),
                    }
                }
                Err(e) => println!("tcp.connect {} {}", addr.as_ref(), e),
            }
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
