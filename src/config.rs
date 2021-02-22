use clap::Clap;
use std::net::SocketAddr;

#[derive(Clap, Debug)]
#[clap(version = "0.1", author = "Lazy Mechanic")]
pub struct Config {
    /// Period in seconds. How often to send messages
    #[clap(long, default_value = "10")]
    pub period: u64,

    /// Port on which to start client
    #[clap(long)]
    pub port: u16,

    /// Another client address. Use if you want to connect to the network.
    /// If omitted, then new network will be launched
    #[clap(long)]
    pub connect: Option<SocketAddr>,
}

impl Config {
    pub fn parse_args() -> Config {
        Config::parse()
    }
}
