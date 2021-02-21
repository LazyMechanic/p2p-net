mod config;

use clap::Clap;
use config::Config;

#[tokio::main]
async fn main() {
    let cfg = Config::parse_args();
    println!("{:#?}", cfg);
}
