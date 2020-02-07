#![allow(non_snake_case)]
#![allow(unused_imports)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
use crate::codec::mqtt::*;
use crate::server::*;
use bytes::Bytes;
use bytestring::ByteString;
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
        let tm = Duration::from_secs(30);
        loop {
            if let Ok(Ok(socket)) = timeout(tm, TcpStream::connect(addr.as_ref())).await {
                let codec = Framed::new(socket, MqttCodec::new());
                let mut peer = Peer::from(client_id.clone(), codec);
                peer.set_keep_alive(Duration::from_secs(30));
                let mut c = Client { tx: None };
                if let Ok(_) = peer
                    .connect(
                        last_will.clone(),
                        username.clone(),
                        password.clone(),
                        |_p, tx| c.tx = Some(tx),
                    )
                    .await
                {
                    tokio::spawn(async move {
                        if let Err(e) = peer.process_loop(|_packet| -> bool { true }).await {
                            println!(
                                "failed to process connection {}; error = {}",
                                peer.client_id, e
                            );
                        }
                    });
                    return Ok(c);
                }
            }
            println!("reconnect");
        }
    }

    pub fn publish(&self, publish: Publish) -> Result<()> {
        if let Some(tx) = &self.tx {
            if let Err(_e) = tx.send(Message::Forward(publish)) {
                return Err(Box::new(ParseError::InvalidClientId));
            }
        }
        Ok(())
    }
}
