use crate::prelude::*;
use rand::distributions::Alphanumeric;
use rand::Rng;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    ConnectionInfo(ConnectionInfo),
    NewConnection(NewConnection),
    NetworkState(NetworkState),
    Text(Text),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConnection {
    pub addr: SocketAddr,
}

impl NewConnection {
    pub fn new(addr: SocketAddr) -> NewConnection {
        NewConnection { addr }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub port: u16,
}

impl ConnectionInfo {
    pub fn new(port: u16) -> ConnectionInfo {
        ConnectionInfo { port }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkState {
    pub peers: HashMap<SocketAddr, peer::Info>,
}

impl NetworkState {
    pub fn new(peers: HashMap<SocketAddr, peer::Info>) -> NetworkState {
        NetworkState { peers }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub text: String,
}

impl Text {
    pub fn new<S>(text: S) -> Text
    where
        S: Into<String>,
    {
        Text { text: text.into() }
    }

    pub fn with_random_text() -> Text {
        let text: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();

        Text { text }
    }
}
