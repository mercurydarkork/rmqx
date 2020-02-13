use crate::codec::mqtt::*;
use bytes::{Buf, BytesMut};
use futures_sink::Sink;
use pin_project::pin_project;
use std::io;
use std::marker::Unpin;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::stream::Stream;
// use tokio::stream::StreamExt;
use futures::{SinkExt, StreamExt};

use tokio_util::codec::{Decoder, Encoder};

#[pin_project]
pub struct MqttStream<T> {
    #[pin]
    io: T,
    codec: MqttCodec,
    pub session_present: bool,
    connect: bool,
    eof: bool,
    is_readable: bool,
    rbuffer: BytesMut,
    wbuffer: BytesMut,
}

const INITIAL_CAPACITY: usize = 4 * 1024;
const BACKPRESSURE_BOUNDARY: usize = INITIAL_CAPACITY;

impl<T> MqttStream<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    fn new(io: T) -> Self {
        Self {
            io: io,
            codec: MqttCodec::new(),
            session_present: false,
            connect: false,
            eof: false,
            is_readable: false,
            rbuffer: BytesMut::with_capacity(INITIAL_CAPACITY),
            wbuffer: BytesMut::with_capacity(INITIAL_CAPACITY),
        }
    }

    pub async fn accept<F>(io: T, f: F) -> Result<Self, MqttError>
    where
        F: FnOnce(Connect) -> bool,
    {
        let mut stream = Self::new(io);
        if let Some(Ok(Packet::Connect(connect))) = stream.next().await {
            if f(connect) {
                let ack = Packet::ConnectAck {
                    session_present: true,
                    return_code: ConnectCode::ConnectionAccepted,
                };
                stream.send(ack).await?;
                return Ok(stream);
            }
        }
        Err(MqttError::AuthError)
    }

    pub async fn connect(io: T, conn: Connect) -> Result<Self, MqttError> {
        let mut stream = Self::new(io);
        stream.send(Packet::Connect(conn)).await?;
        if let Some(Ok(Packet::ConnectAck {
            session_present,
            return_code,
        })) = stream.next().await
        {
            match return_code {
                ConnectCode::ConnectionAccepted => {
                    stream.session_present = session_present;
                    return Ok(stream);
                }
                ConnectCode::UnacceptableProtocolVersion => {
                    return Err(MqttError::InvalidProtocol);
                }
                ConnectCode::BadUserNameOrPassword => return Err(MqttError::AuthError),
                ConnectCode::IdentifierRejected => return Err(MqttError::InvalidClientId),
                ConnectCode::NotAuthorized => return Err(MqttError::AuthError),
                ConnectCode::ServiceUnavailable => return Err(MqttError::ServiceUnavailable),
                _ => return Err(MqttError::AuthError),
            }
        }
        Err(MqttError::AuthError)
    }
}

impl<T: AsyncRead> Stream for MqttStream<T> {
    type Item = Result<Packet, MqttError>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut pinned = self.project();
        loop {
            if *pinned.is_readable {
                if *pinned.eof {
                    let frame = pinned.codec.decode_eof(&mut pinned.rbuffer)?;
                    return Poll::Ready(frame.map(Ok));
                }
                if let Some(frame) = pinned.codec.decode(&mut pinned.rbuffer)? {
                    return Poll::Ready(Some(Ok(frame)));
                }

                *pinned.is_readable = false;
            }
            assert!(!*pinned.eof);
            pinned.rbuffer.reserve(1);
            let bytect = match pinned.io.as_mut().poll_read_buf(cx, &mut pinned.rbuffer)? {
                Poll::Ready(ct) => ct,
                Poll::Pending => return Poll::Pending,
            };
            if bytect == 0 {
                *pinned.eof = true;
            }
            *pinned.is_readable = true;
        }
    }
}

impl<T: AsyncWrite> Sink<Packet> for MqttStream<T> {
    type Error = MqttError;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        if self.wbuffer.len() >= BACKPRESSURE_BOUNDARY {
            match self.as_mut().poll_flush(cx) {
                Poll::Pending => return Poll::Pending,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Ready(Ok(())) => (),
            };

            if self.wbuffer.len() >= BACKPRESSURE_BOUNDARY {
                return Poll::Pending;
            }
        }
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Packet) -> Result<(), Self::Error> {
        let mut pinned = self.project();
        pinned.codec.encode(item, &mut pinned.wbuffer)?;
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut pinned = self.project();
        while !pinned.wbuffer.is_empty() {
            let buf = &pinned.wbuffer;
            let n = ready!(pinned.io.as_mut().poll_write(cx, &buf))?;

            if n == 0 {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "failed to \
                     write frame to transport",
                )
                .into()));
            }

            pinned.wbuffer.advance(n);
        }

        // Try flushing the underlying IO
        ready!(pinned.io.poll_flush(cx))?;

        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_flush(cx))?;
        ready!(self.project().io.poll_shutdown(cx))?;
        Poll::Ready(Ok(()))
    }
}

impl<T> std::fmt::Debug for MqttStream<T>
where
    T: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttStream")
            .field("io", &self.io)
            .field("codec", &self.codec)
            .field("read_buffer", &self.rbuffer)
            .field("write_buffer", &self.wbuffer)
            .finish()
    }
}
