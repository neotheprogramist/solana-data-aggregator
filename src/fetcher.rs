use std::str::FromStr;

use futures::{future::join_all, StreamExt};
use serde::{Deserialize, Serialize};
use solana_client::{
    nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient},
    pubsub_client::PubsubClientError,
    rpc_config::{RpcBlockConfig, RpcTransactionConfig},
};
use solana_sdk::{commitment_config::CommitmentConfig, signature::Signature};
use solana_transaction_status_client_types::{
    EncodedConfirmedTransactionWithStatusMeta, TransactionDetails, UiTransactionEncoding,
};
use surrealdb::{Connection, RecordId, Surreal};
use thiserror::Error;
use tokio::{select, sync::broadcast};
use tracing::{error, info};
use url::Url;

#[derive(Debug, Error)]
pub enum TransactionFetcherError {
    #[error(transparent)]
    RpcClient(#[from] solana_client::client_error::ClientError),

    #[error(transparent)]
    PubsubClient(#[from] PubsubClientError),

    #[error(transparent)]
    Surrealdb(#[from] surrealdb::Error),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Transaction {
    signature: String,
    slot: u64,
    block_hash: String,
    timestamp: i64,
    data: EncodedConfirmedTransactionWithStatusMeta,
}

pub struct TransactionFetcher<C: Connection> {
    rpc: RpcClient,
    ws: PubsubClient,
    block_config: RpcBlockConfig,
    transaction_config: RpcTransactionConfig,
    db: Surreal<C>,
    root_lag: u64,
    tx_limit: usize,
    shutdown: broadcast::Receiver<()>,
}

impl<C: Connection> TransactionFetcher<C> {
    pub async fn new(
        rpc_url: Url,
        ws_url: Url,
        db: Surreal<C>,
        root_lag: u64,
        tx_limit: usize,
        shutdown: broadcast::Receiver<()>,
    ) -> Result<Self, TransactionFetcherError> {
        let rpc =
            RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::finalized());
        let ws = PubsubClient::new(ws_url.as_str()).await?;
        let block_config = RpcBlockConfig {
            encoding: Some(UiTransactionEncoding::JsonParsed),
            transaction_details: Some(TransactionDetails::Signatures),
            rewards: Some(true),
            commitment: None,
            max_supported_transaction_version: Some(0),
        };
        let transaction_config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::JsonParsed),
            commitment: Some(CommitmentConfig::finalized()),
            max_supported_transaction_version: Some(0),
        };

        Ok(Self {
            rpc,
            ws,
            block_config,
            transaction_config,
            db,
            root_lag,
            tx_limit,
            shutdown,
        })
    }

    pub async fn run(&mut self) -> Result<(), TransactionFetcherError> {
        let (mut stream, unsubscribe) = self.ws.root_subscribe().await?;

        loop {
            select! {
                Some(slot) = stream.next() => {
                    let adjusted_slot = slot.saturating_sub(self.root_lag);
                    match self.rpc.get_block_with_config(adjusted_slot, self.block_config).await {
                        Ok(block) => {
                            if let Some(signatures) = block.signatures {
                                let mut fetch_futures = Vec::new();

                                for signature in signatures.into_iter().take(self.tx_limit) {
                                    let rpc = &self.rpc;
                                    let db = &self.db;
                                    let transaction_config = self.transaction_config;
                                    let block_hash = block.blockhash.clone();
                                    let slot = adjusted_slot;

                                    fetch_futures.push(async move {
                                        match Signature::from_str(&signature) {
                                            Ok(s) => match rpc.get_transaction_with_config(&s, transaction_config).await {
                                                Ok(transaction) => {
                                                    let content = Transaction {
                                                        signature,
                                                        slot,
                                                        block_hash,
                                                        timestamp: block.block_time.unwrap_or_default(),
                                                        data: transaction,
                                                    };

                                                    #[derive(Debug, Deserialize)]
                                                    struct Id {
                                                        #[allow(dead_code)]
                                                        id: RecordId,
                                                    }

                                                    match db.create::<Option<Id>>("transactions").content(content).await {
                                                        Ok(id) => info!("Stored transaction: {:?}", id),
                                                        Err(e) => error!("Failed to store transaction: {}", e),
                                                    }
                                                }
                                                Err(e) => error!("Failed to fetch transaction details: {}", e),
                                            },
                                            Err(e) => error!("Invalid signature format: {}", e),
                                        }
                                    });
                                }

                                if !fetch_futures.is_empty() {
                                    join_all(fetch_futures).await;
                                }
                            }
                        }
                        Err(e) => error!("Failed to fetch block data: {}", e),
                    }
                },
                Ok(_) = self.shutdown.recv() => {
                    info!("Shutdown signal received.");
                    break;
                }
                else => {
                    error!("Unexpected error in fetch loop, shutting down.");
                    break;
                }
            }
        }

        info!("Fetcher shutting down...");
        unsubscribe().await;
        info!("Fetcher shut down.");
        Ok(())
    }
}
