use crate::prelude::*;

use std::collections::HashMap;
use std::ops::DerefMut;
use tokio_serde::formats::*;
use tokio_util::codec::LengthDelimitedCodec;

type Peers = HashMap<SocketAddr, Peer>;

pub struct Context {
    port: u16,
    peers: RwLock<Peers>,
    sleep_duration: Duration,
}

impl Context {
    pub fn new(port: u16, period: u64) -> Context {
        Context {
            port,
            peers: Default::default(),
            sleep_duration: Duration::from_secs(period),
        }
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn sleep_duration(&self) -> Duration {
        self.sleep_duration
    }

    pub async fn add_peer(&self, addr: SocketAddr, peer: Peer) -> anyhow::Result<()> {
        log::debug!("add peer: addr={}, peer={:?}", addr, peer);

        let mut peers = self.peers.write().await;
        if peers.contains_key(&addr) {
            return Err(anyhow!("peer already exists"));
        }

        peers.insert(addr, peer);

        Ok(())
    }

    pub async fn update_peer_info(
        &self,
        addr: SocketAddr,
        info: Option<peer::Info>,
    ) -> anyhow::Result<()> {
        let mut peers = self.peers.write().await;
        let peer = peers
            .get_mut(&addr)
            .ok_or_else(|| anyhow!(format!("peer with addr={} not found", addr)))?;

        peer.info = info;

        Ok(())
    }

    pub async fn disconnect(&self, addr: SocketAddr) -> anyhow::Result<()> {
        let mut peers = self.peers.write().await;
        if !peers.contains_key(&addr) {
            return Err(anyhow!(format!("peer with addr={} not found", addr)));
        }

        peers.remove(&addr);

        Ok(())
    }

    pub async fn has_peers(&self) -> bool {
        let peers = self.peers.read().await;
        !peers.is_empty()
    }

    pub async fn network_state(&self) -> HashMap<SocketAddr, peer::Info> {
        let peers = self.peers.read().await;
        let res = peers
            .iter()
            .filter(|(&addr, peer)| peer.info.is_some())
            .map(|(&addr, peer)| (addr, peer.info.clone().unwrap()))
            .collect();

        log::debug!("network state: {:?}", res);
        res
    }

    pub async fn broadcast(&self, msg: Message) {
        let mut peers = self.peers.write().await;

        log::debug!("peers count: {}", peers.len());
        log::debug!("peers: {:?}", peers);
        for (_, peer) in peers.iter_mut() {
            if let Err(e) = self.send_impl(&mut peer.tx, msg.clone()).await {
                log::error!("error occurred on send: {:?}", e)
            }
        }
    }

    pub async fn broadcast_with_exclude(&self, msg: Message, exclude_addr: SocketAddr) {
        let mut peers = self.peers.write().await;

        // Filter peers which not contains in `exclude_addrs`
        for (_, peer) in peers
            .iter_mut()
            .filter(|(addr, peer)| **addr != exclude_addr)
        {
            if let Err(e) = self.send_impl(&mut peer.tx, msg.clone()).await {
                log::error!("error occurred on send: {:?}", e)
            }
        }
    }

    pub async fn broadcast_with_excludes(&self, msg: Message, exclude_addrs: Vec<SocketAddr>) {
        let mut peers = self.peers.write().await;

        // Filter peers with non none info and which not contains in `exclude_addrs`
        for (_, peer) in peers
            .iter_mut()
            .filter(|(addr, peer)| peer.info.is_some() && !exclude_addrs.contains(*addr))
        {
            if let Err(e) = self.send_impl(&mut peer.tx, msg.clone()).await {
                log::error!("error occurred on send: {:?}", e)
            }
        }
    }

    pub async fn send(&self, addr: SocketAddr, msg: Message) -> anyhow::Result<()> {
        let mut peers = self.peers.write().await;
        let peer = peers
            .get_mut(&addr)
            .ok_or_else(|| anyhow!(format!("peer with addr={} not found", addr)))?;

        self.send_impl(&mut peer.tx, msg).await
    }

    async fn send_impl(&self, tx: &mut SocketTx, msg: Message) -> anyhow::Result<()> {
        let length_delimited_transport = LengthDelimitedCodec::builder().new_write(tx);

        let mut serializer = tokio_serde::SymmetricallyFramed::new(
            length_delimited_transport,
            SymmetricalJson::<Message>::default(),
        );

        serializer.send(msg).await?;

        Ok(())
    }
}
