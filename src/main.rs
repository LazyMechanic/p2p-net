pub mod config;
pub mod context;
pub mod event;
pub mod message;
pub mod peer;
pub mod prelude;
pub mod server;

use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run().await
}

async fn run() -> anyhow::Result<()> {
    init_logger();

    let cfg = Config::parse_args();
    log::info!("start with config={:?}", cfg);

    server::run(cfg).await?;

    Ok(())
}

fn init_logger() {
    let log_filters = std::env::var("RUST_LOG").unwrap_or_default();

    env_logger::Builder::new()
        .parse_filters(&log_filters)
        .format(|formatter, record| {
            use std::io::Write;

            writeln!(
                formatter,
                "{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init()
}
