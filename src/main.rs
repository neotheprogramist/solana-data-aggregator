use std::net::SocketAddr;

use clap::Parser;
use fetcher::{TransactionFetcher, TransactionFetcherError};
use surrealdb::{engine::remote::ws::Ws, opt::auth::Root, Surreal};
use thiserror::Error;
use tokio::{join, net::TcpListener, sync::broadcast};
use tracing::{error, info, Level};
use url::Url;

pub mod fetcher;
pub mod server;
pub mod shutdown;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, short, env)]
    rpc_url: Url,

    #[arg(long, short, env)]
    ws_url: Url,

    #[arg(long, short, env)]
    bind: SocketAddr,

    #[arg(long, short = 'a', env)]
    db_addr: String,

    #[arg(long, short = 'u', env)]
    db_user: String,

    #[arg(long, short = 'p', env)]
    db_pass: String,

    #[arg(long, env)]
    db_ns: String,

    #[arg(long, env)]
    db_db: String,

    #[arg(long, short = 'l', env, default_value_t = 100)]
    root_lag: u64,

    #[arg(long, short, env, default_value_t = 1000)]
    tx_limit: usize,
}

#[derive(Debug, Error)]
enum AppError {
    #[error(transparent)]
    Surrealdb(#[from] surrealdb::Error),

    #[error(transparent)]
    TransactionFetcher(#[from] TransactionFetcherError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    let args = Args::parse();
    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    info!("Starting server on {}", args.bind);
    let listener = TcpListener::bind(args.bind).await?;

    info!("Connecting to database at {}", args.db_addr);
    let db = Surreal::new::<Ws>(&args.db_addr).await?;
    db.signin(Root {
        username: &args.db_user,
        password: &args.db_pass,
    })
    .await?;
    db.use_ns(&args.db_ns).use_db(&args.db_db).await?;

    let mut transaction_fetcher = TransactionFetcher::new(
        args.rpc_url,
        args.ws_url,
        db.clone(),
        args.root_lag,
        args.tx_limit,
        shutdown_tx.subscribe(),
    )
    .await?;

    info!("Starting tasks...");
    let result = join!(
        transaction_fetcher.run(),
        server::run(listener, db, shutdown_tx.subscribe()),
        async {
            shutdown::shutdown_signal().await;
            if let Err(e) = shutdown_tx.send(()) {
                error!("Failed to send shutdown signal: {}", e);
            }
        }
    );

    info!("Shutdown complete: {:?}", result);

    Ok(())
}
