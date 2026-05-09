//! MCP Sampling manager with backpressure and graceful shutdown.
//!
//! Manages sampling requests from the PDF MCP server, implementing bounded
//! channels for backpressure and cancellation tokens for graceful shutdown.

use super::client::{SamplingClient, SamplingClientConfig};
use super::{SamplingRequest, SamplingResponse};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub use super::client::OutgoingRequest;

#[derive(Debug, Error)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum SamplingError {
    #[error("Service overloaded: {0}")]
    Overloaded(String),

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Response timeout")]
    ResponseTimeout,

    #[error("Service shutting down")]
    ShuttingDown,

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Client does not support sampling")]
    ClientNotSupported,
}

#[allow(dead_code)]
struct SamplingTask {
    request: SamplingRequest,
    response_tx: tokio::sync::oneshot::Sender<Result<SamplingResponse, SamplingError>>,
}

#[allow(dead_code)]
pub struct SamplingManager {
    request_tx: mpsc::Sender<SamplingTask>,
    cancel_token: CancellationToken,
    active_requests: Arc<AtomicUsize>,
    max_requests: usize,
    outgoing_tx: Option<mpsc::Sender<OutgoingRequest>>,
    pending: PendingRequests,
}

type PendingRequests =
    Arc<tokio::sync::RwLock<HashMap<u64, tokio::sync::oneshot::Sender<Result<SamplingResponse, SamplingError>>>>>;

#[allow(dead_code)]
impl SamplingManager {
    pub fn new(capacity: usize, max_requests: usize) -> Self {
        let (request_tx, mut request_rx) = mpsc::channel::<SamplingTask>(capacity);
        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();
        let active_requests = Arc::new(AtomicUsize::new(0));
        let active_requests_clone = Arc::clone(&active_requests);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(task) = request_rx.recv() => {
                        let active = Arc::clone(&active_requests_clone);
                        tokio::spawn(async move {
                            let response = Err(SamplingError::Internal(
                                "Sampling requires MCP client support. Use SamplingManager::with_client() for full functionality.".to_string()
                            ));
                            let _ = task.response_tx.send(response);
                            active.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                    _ = cancel_token_clone.cancelled() => {
                        tracing::info!("Sampling manager shutting down gracefully");
                        break;
                    }
                    else => break,
                }
            }
        });

        Self {
            request_tx,
            cancel_token,
            active_requests,
            max_requests,
            outgoing_tx: None,
            pending: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub fn with_client(
        capacity: usize,
        config: SamplingClientConfig,
    ) -> (Self, mpsc::Receiver<OutgoingRequest>) {
        let (request_tx, mut request_rx) = mpsc::channel::<SamplingTask>(capacity);
        let (outgoing_tx, outgoing_rx) = mpsc::channel::<OutgoingRequest>(capacity);
        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();
        let active_requests = Arc::new(AtomicUsize::new(0));
        let active_requests_clone = Arc::clone(&active_requests);
        let max_requests = config.max_concurrent;
        let timeout_secs = config.timeout_secs;

        let outgoing_tx_clone = outgoing_tx.clone();
        let request_id_counter = Arc::new(AtomicU64::new(1));
        let pending_map: PendingRequests = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        let pending_clone = Arc::clone(&pending_map);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(task) = request_rx.recv() => {
                        let outgoing_tx = outgoing_tx_clone.clone();
                        let active = Arc::clone(&active_requests_clone);
                        let id_counter = Arc::clone(&request_id_counter);
                        let pending = Arc::clone(&pending_clone);
                        tokio::spawn(async move {
                            let req_id = id_counter.fetch_add(1, Ordering::Relaxed);

                            let outgoing = OutgoingRequest {
                                id: req_id,
                                request: task.request,
                            };

                            if outgoing_tx.send(outgoing).await.is_err() {
                                let _ = task.response_tx.send(Err(SamplingError::ChannelClosed));
                                active.fetch_sub(1, Ordering::Relaxed);
                                return;
                            }

                            // Store the response sender in the pending map and wait
                            // for the response or timeout, whichever comes first.
                            let (response_tx_2, response_rx) = tokio::sync::oneshot::channel();
                            {
                                let mut p = pending.write().await;
                                p.insert(req_id, response_tx_2);
                            }

                            let timeout_duration = std::time::Duration::from_secs(timeout_secs);
                            match tokio::time::timeout(timeout_duration, response_rx).await {
                                Ok(Ok(Ok(response))) => {
                                    let _ = task.response_tx.send(Ok(response));
                                }
                                Ok(Ok(Err(e))) => {
                                    let _ = task.response_tx.send(Err(e));
                                }
                                Ok(Err(_)) | Err(_) => {
                                    let mut p = pending.write().await;
                                    p.remove(&req_id);
                                    let _ = task.response_tx.send(Err(SamplingError::ResponseTimeout));
                                }
                            }
                            active.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                    _ = cancel_token_clone.cancelled() => {
                        tracing::info!("Sampling manager shutting down gracefully");
                        break;
                    }
                    else => break,
                }
            }
        });

        (
            Self {
                request_tx,
                cancel_token,
                active_requests,
                max_requests,
                outgoing_tx: Some(outgoing_tx),
                pending: pending_map,
            },
            outgoing_rx,
        )
    }

    pub fn with_sampling_client(
        capacity: usize,
        max_requests: usize,
        client: Arc<SamplingClient>,
    ) -> Self {
        let (request_tx, mut request_rx) = mpsc::channel::<SamplingTask>(capacity);
        let cancel_token = CancellationToken::new();
        let cancel_token_clone = cancel_token.clone();
        let active_requests = Arc::new(AtomicUsize::new(0));
        let active_requests_clone = Arc::clone(&active_requests);

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(task) = request_rx.recv() => {
                        let client = Arc::clone(&client);
                        let active = Arc::clone(&active_requests_clone);
                        tokio::spawn(async move {
                            let result = client.request_sampling(task.request).await;
                            let _ = task.response_tx.send(result);
                            active.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                    _ = cancel_token_clone.cancelled() => {
                        tracing::info!("Sampling manager shutting down gracefully");
                        break;
                    }
                    else => break,
                }
            }
        });

        Self {
            request_tx,
            cancel_token,
            active_requests,
            max_requests,
            outgoing_tx: None,
            pending: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    pub async fn request_sampling(
        &self,
        request: SamplingRequest,
    ) -> Result<SamplingResponse, SamplingError> {
        if self.cancel_token.is_cancelled() {
            return Err(SamplingError::ShuttingDown);
        }

        let current = self.active_requests.fetch_add(1, Ordering::Relaxed);
        if current >= self.max_requests {
            self.active_requests.fetch_sub(1, Ordering::Relaxed);
            return Err(SamplingError::Overloaded(
                "Service temporarily unavailable due to high load".to_string(),
            ));
        }

        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        let result = self
            .request_tx
            .send(SamplingTask {
                request,
                response_tx,
            })
            .await
            .map_err(|_| {
                self.active_requests.fetch_sub(1, Ordering::Relaxed);
                SamplingError::ChannelClosed
            });

        match result {
            Ok(_) => response_rx
                .await
                .map_err(|_| SamplingError::ResponseTimeout)?,
            Err(e) => Err(e),
        }
    }

    pub fn shutdown(&self) {
        self.cancel_token.cancel();
    }

    pub fn is_shutting_down(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    pub fn active_request_count(&self) -> usize {
        self.active_requests.load(Ordering::Relaxed)
    }

    pub fn outgoing_sender(&self) -> Option<mpsc::Sender<OutgoingRequest>> {
        self.outgoing_tx.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sampling_manager_backpressure() {
        let manager = SamplingManager::new(2, 2);

        let req1 = manager.request_sampling(SamplingRequest::default());
        let req2 = manager.request_sampling(SamplingRequest::default());

        let _ = req1.await;
        let _ = req2.await;

        let req3 = manager.request_sampling(SamplingRequest::default());
        assert!(!matches!(req3.await, Err(SamplingError::Overloaded(_))));
    }

    #[tokio::test]
    async fn test_graceful_shutdown() {
        let manager = SamplingManager::new(10, 5);
        manager.shutdown();

        let result = manager.request_sampling(SamplingRequest::default()).await;
        assert!(matches!(result, Err(SamplingError::ShuttingDown)));
    }

    #[tokio::test]
    async fn test_active_request_count() {
        let manager = SamplingManager::new(10, 5);
        assert_eq!(manager.active_request_count(), 0);
        assert!(!manager.is_shutting_down());
    }

    #[tokio::test]
    async fn test_with_client() {
        let config = SamplingClientConfig::default();
        let (manager, _rx) = SamplingManager::with_client(10, config);
        assert_eq!(manager.active_request_count(), 0);
    }
}
