use crate::config::Config;
use crate::message::Message;
use futures::prelude::*;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};
use tokio::time::Duration;
use tokio_util::codec::length_delimited::Builder;
use tokio_util::codec::{Decoder, Framed, LengthDelimitedCodec, LinesCodec};

struct Context {
    pub addr: SocketAddr,
    pub connections: RwLock<HashMap<SocketAddr, OwnedWriteHalf>>,
    pub period: Duration,
}

impl Context {
    pub fn new(addr: SocketAddr, period: u64) -> Context {
        Context {
            addr,
            connections: Default::default(),
            period: Duration::from_secs(period),
        }
    }

    pub async fn insert_connection(self: &Arc<Self>, socket: TcpStream, addr: SocketAddr) {
        // TODO: send connected event to others

        let (rx, tx) = socket.into_split();

        log::info!("connected: addr={}", addr);

        // Add new client
        self.connections.write().await.insert(addr, tx);

        // Process
        tokio::spawn(process(rx, addr, Arc::clone(self)));
    }
}

fn codec_builder() -> Builder {
    let mut builder = LengthDelimitedCodec::builder();
    builder.length_field_length(8);

    builder
}

async fn process(mut rx: OwnedReadHalf, addr: SocketAddr, ctx: Arc<Context>) {
    let mut framed_read = codec_builder().new_read(rx);

    while let Some(buf) = framed_read.next().await {
        match buf {
            Ok(buf) => {
                let msg = match Message::try_from_bytes(&buf.as_ref()[8..]) {
                    Ok(msg) => msg,
                    Err(e) => {
                        log::error!("{:?}", e);
                        continue;
                    }
                };

                log::info!("received: addr={}, msg={:?}", addr, msg);
            }
            Err(e) => {
                log::error!("{:?}", e);
                return;
            }
        }
    }
}

async fn write_random_message(ctx: Arc<Context>) {
    loop {
        let msg = Message::with_random_text();

        for (addr, tx) in ctx.connections.write().await.iter_mut() {
            let mut framed_write = codec_builder().new_write(tx);

            log::info!("sended: addr={}, msg={:?}", ctx.addr, msg);

            let b = bytes::Bytes::from(msg.to_bytes());
            if let Err(e) = framed_write.send(b).await {
                log::error!("error on send: {:?}", e);
                // TODO: remove client
            }
        }

        tokio::time::sleep(ctx.period).await;
    }
}

pub async fn run(cfg: Config) -> anyhow::Result<()> {
    let addr = format!("0.0.0.0:{}", cfg.port).parse::<SocketAddr>()?;
    let listener = TcpListener::bind(addr).await?;
    let ctx = Arc::new(Context::new(addr, cfg.period));

    if let Some(connection_addr) = cfg.connect {
        let socket = TcpStream::connect(connection_addr).await?;
        ctx.insert_connection(socket, connection_addr).await;
    };

    tokio::spawn(write_random_message(Arc::clone(&ctx)));

    loop {
        let (socket, socket_addr) = listener.accept().await?;
        ctx.insert_connection(socket, socket_addr).await;
    }
}
