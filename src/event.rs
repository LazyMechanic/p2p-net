use crate::prelude::*;

#[derive(Debug)]
pub enum Event {
    BroadcastMessage(Message),
    BroadcastWithExcludeMessage {
        exclude: SocketAddr,
        msg: Message,
    },
    SendMessage {
        to: SocketAddr,
        msg: Message,
    },
    RecvText {
        from: SocketAddr,
        msg: message::Text,
    },
    RecvConnectionInfo {
        from: SocketAddr,
        msg: message::ConnectionInfo,
    },
    UpdatePeerInfo {
        addr: SocketAddr,
        msg: message::ConnectionInfo,
    },
    RecvNewConnection(message::NewConnection),
    RecvNetworkState(message::NetworkState),
    AcceptConnection {
        socket: TcpStream,
        addr: SocketAddr,
    },
    Connect(SocketAddr),
    Disconnect(SocketAddr),
}
