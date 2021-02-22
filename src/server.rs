use crate::prelude::*;

use tokio_serde::formats::*;
use tokio_util::codec::LengthDelimitedCodec;

async fn handle_events(
    ctx: Arc<Context>,
    event_tx: EventTx,
    mut event_rx: EventRx,
) -> anyhow::Result<()> {
    while let Some(event) = event_rx.recv().await {
        match event {
            Event::BroadcastMessage(msg) => {
                if ctx.has_peers().await {
                    log::info!("broadcast message: msg={:?}", msg);
                    ctx.broadcast(msg).await;
                }
            }
            Event::BroadcastWithExcludeMessage { exclude, msg } => {
                if ctx.has_peers().await {
                    log::info!("broadcast message: msg={:?}", msg);
                    ctx.broadcast_with_exclude(msg, exclude).await;
                }
            }
            Event::SendMessage { to, msg } => {
                log::info!("send message: to={}, msg={:?}", to, msg);
                if let Err(err) = ctx.send(to, msg).await {
                    log::error!("error occurred on send: {}", err);
                }
            }
            Event::RecvText { from, msg } => {
                log::info!("received text: from={}, msg={:?}", from, msg)
            }
            Event::RecvConnectionInfo { from, msg } => {
                log::info!("received connection info: from={} msg={:?}", from, msg);

                // Send network state
                let event = Event::SendMessage {
                    to: from,
                    msg: Message::NetworkState(message::NetworkState {
                        peers: ctx.network_state().await,
                    }),
                };
                if let Err(err) = event_tx.send(event) {
                    log::error!("error occurred on send event: {}", err);
                    continue;
                }

                // Update peer info
                let event = Event::UpdatePeerInfo {
                    addr: from,
                    msg: msg.clone(),
                };
                if let Err(err) = event_tx.send(event) {
                    log::error!("error occurred on send event: {}", err);
                    continue;
                }
            }
            Event::UpdatePeerInfo { addr, msg } => {
                log::info!("update peer info: addr={}, msg={:?}", addr, msg);

                if let Err(err) = ctx
                    .update_peer_info(addr, Some(peer::Info { port: msg.port }))
                    .await
                {
                    log::error!("error occurred on update peer info: {}", err);
                    continue;
                }
            }
            Event::RecvNewConnection(msg) => {
                log::info!("received new connection: addr={}", msg.addr);

                let socket = match TcpStream::connect(msg.addr).await {
                    Ok(s) => s,
                    Err(err) => {
                        log::error!("error occurred on connect: {}", err);
                        continue;
                    }
                };

                if let Err(err) =
                    handle_connection(Arc::clone(&ctx), event_tx.clone(), socket, msg.addr, None)
                        .await
                {
                    log::error!("error occurred on handle connection: {}", err)
                }
            }
            Event::RecvNetworkState(msg) => {
                log::info!("received network state: size={}", msg.peers.len());
                log::debug!("state: {:?}", msg);

                for (addr, info) in msg.peers {
                    let event = Event::Connect(SocketAddr::new(addr.ip(), info.port));
                    if let Err(err) = event_tx.send(event) {
                        log::error!("error occurred on send event: {}", err);
                        continue;
                    }
                }
            }
            Event::AcceptConnection { mut socket, addr } => {
                log::info!("accept new client: addr={}", addr);

                // Add new peer and handle messages
                if let Err(err) =
                    handle_connection(Arc::clone(&ctx), event_tx.clone(), socket, addr, None).await
                {
                    log::error!("error occurred on handle connection: {}", err)
                }
            }
            Event::Disconnect(_) => todo!(),
            Event::Connect(addr) => {
                log::info!("connect: dest_addr={}", addr);

                // Connect to network
                let mut socket = match TcpStream::connect(addr).await {
                    Ok(s) => s,
                    Err(err) => {
                        log::error!("error occurred on connect: {}", err);
                        continue;
                    }
                };

                // Add new peer and handle messages
                let info = peer::Info { port: addr.port() };
                if let Err(err) =
                    handle_connection(Arc::clone(&ctx), event_tx.clone(), socket, addr, Some(info))
                        .await
                {
                    log::error!("error occurred on handle connection: {}", err);
                    ctx.exit().await;
                    continue;
                }
            }
        }
    }

    Ok(())
}

async fn handle_connection(
    ctx: Arc<Context>,
    event_tx: EventTx,
    mut socket: TcpStream,
    addr: SocketAddr,
    info: Option<peer::Info>,
) -> anyhow::Result<()> {
    // Split socket to RX and TX
    let (socket_rx, socket_tx) = socket.into_split();

    // Add new peer without info
    ctx.add_peer(
        addr,
        Peer {
            tx: socket_tx,
            info,
        },
    )
    .await;

    let length_delimited_transport = LengthDelimitedCodec::builder().new_read(socket_rx);

    let mut deserializer = tokio_serde::SymmetricallyFramed::new(
        length_delimited_transport,
        SymmetricalJson::<Message>::default(),
    );

    tokio::spawn(async move {
        while let Some(msg) = deserializer.next().await {
            match msg {
                Ok(msg) => {
                    let event = match msg {
                        Message::Text(msg) => Event::RecvText { from: addr, msg },
                        Message::NetworkState(msg) => Event::RecvNetworkState(msg),
                        Message::NewConnection(msg) => Event::RecvNewConnection(msg),
                        Message::ConnectionInfo(msg) => {
                            Event::RecvConnectionInfo { from: addr, msg }
                        }
                    };

                    if let Err(err) = event_tx.send(event) {
                        log::error!("error occurred on send event: {}", err);
                        continue;
                    }
                }
                Err(err) => {
                    log::error!("error occurred on deserialize message: {}", err);
                    ctx.exit().await;
                }
            }
        }
    });

    Ok(())
}

async fn handle_connection_v2(
    event_tx: EventTx,
    socket_rx: SocketRx,
    addr: SocketAddr,
) -> anyhow::Result<()> {
    let length_delimited_transport = LengthDelimitedCodec::builder().new_read(socket_rx);

    let mut deserializer = tokio_serde::SymmetricallyFramed::new(
        length_delimited_transport,
        SymmetricalJson::<Message>::default(),
    );

    while let Some(msg) = deserializer.next().await {
        match msg {
            Ok(msg) => {
                let event = match msg {
                    Message::Text(msg) => Event::RecvText { from: addr, msg },
                    Message::NetworkState(msg) => Event::RecvNetworkState(msg),
                    Message::NewConnection(msg) => Event::RecvNewConnection(msg),
                    Message::ConnectionInfo(msg) => Event::RecvConnectionInfo { from: addr, msg },
                };

                if let Err(err) = event_tx.send(event) {
                    log::error!("error occurred on send event: {:?}", err);
                    continue;
                }
            }
            Err(err) => {
                log::error!("error occurred on deserialize message: {:?}", err);
            }
        }
    }

    Ok(())
}

async fn write(event_tx: EventTx, sleep_duration: Duration) -> anyhow::Result<()> {
    loop {
        let msg_inner = message::Text::with_random_text();
        let event = Event::BroadcastMessage(Message::Text(msg_inner));

        if let Err(err) = event_tx.send(event) {
            log::error!("error occurred on broadcast message: {:?}", err)
        }

        tokio::time::sleep(sleep_duration).await;
    }
}

pub async fn run(cfg: Config) -> anyhow::Result<()> {
    let listing_addr = format!("0.0.0.0:{}", cfg.port).parse::<SocketAddr>()?;
    let listener = TcpListener::bind(listing_addr).await?;
    let ctx = Arc::new(Context::new(cfg.port, cfg.period));

    let (event_tx, event_rx): (EventTx, EventRx) = mpsc::unbounded_channel();

    // Start handling events
    tokio::spawn({
        let ctx = Arc::clone(&ctx);
        let event_tx = event_tx.clone();
        async move {
            if let Err(err) = handle_events(ctx, event_tx, event_rx).await {
                log::error!("error occurred on handle events: {:?}", err)
            }
        }
    });

    // Connect to another network
    if let Some(connection_addr) = cfg.connect {
        // Connect
        let event = Event::Connect(connection_addr);
        event_tx.send(event)?;

        // Send info
        let event = Event::SendMessage {
            to: addr,
            msg: Message::ConnectionInfo(message::ConnectionInfo { port: ctx.port() }),
        };
        event_tx.send(event)?;
    };

    // Broadcast to peers
    tokio::spawn({
        let ctx = Arc::clone(&ctx);
        let event_tx = event_tx.clone();
        async move {
            if let Err(err) = write(event_tx, ctx.sleep_duration()).await {
                log::error!("error occurred on write: {:?}", err)
            }
        }
    });

    // Accept connects
    while ctx.is_running().await {
        let (socket, socket_addr) = listener.accept().await?;

        let event = Event::AcceptConnection {
            socket,
            addr: socket_addr,
        };
        if let Err(err) = event_tx.send(event) {
            log::error!("error occurred on connect: {:?}", err);
            continue;
        }
    }

    Ok(())
}
