use crate::prelude::*;

#[derive(Debug)]
pub struct Peer {
    pub tx: SocketTx,
    pub info: Option<Info>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Info {
    pub port: u16,
}
