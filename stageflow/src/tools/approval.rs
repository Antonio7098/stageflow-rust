//! Approval service for human-in-the-loop workflows.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;
use uuid::Uuid;

/// Approval request status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalStatus {
    /// Waiting for approval.
    Pending,
    /// Approved.
    Approved,
    /// Denied.
    Denied,
    /// Timed out.
    TimedOut,
    /// Cancelled.
    Cancelled,
}

/// An approval request.
#[derive(Debug)]
struct ApprovalRequest {
    /// Request ID.
    id: Uuid,
    /// Tool name.
    tool_name: String,
    /// Approval message.
    message: String,
    /// When the request was created.
    created_at: Instant,
    /// Response channel.
    response_tx: Option<oneshot::Sender<bool>>,
}

/// Service for managing approval requests.
#[derive(Default)]
pub struct ApprovalService {
    /// Pending requests.
    requests: RwLock<HashMap<Uuid, ApprovalRequest>>,
}

impl ApprovalService {
    /// Creates a new approval service.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Requests approval for a tool execution.
    ///
    /// Returns a future that resolves when the approval is decided.
    pub async fn request_approval(
        &self,
        tool_name: &str,
        message: &str,
        timeout: Duration,
    ) -> Result<bool, ApprovalStatus> {
        let request_id = Uuid::new_v4();
        let (tx, rx) = oneshot::channel();

        {
            let request = ApprovalRequest {
                id: request_id,
                tool_name: tool_name.to_string(),
                message: message.to_string(),
                created_at: Instant::now(),
                response_tx: Some(tx),
            };
            self.requests.write().insert(request_id, request);
        }

        // Wait for response with timeout
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(approved)) => {
                self.requests.write().remove(&request_id);
                Ok(approved)
            }
            Ok(Err(_)) => {
                // Channel closed
                self.requests.write().remove(&request_id);
                Err(ApprovalStatus::Cancelled)
            }
            Err(_) => {
                // Timeout
                self.requests.write().remove(&request_id);
                Err(ApprovalStatus::TimedOut)
            }
        }
    }

    /// Approves a pending request.
    pub fn approve(&self, request_id: Uuid) -> bool {
        if let Some(mut request) = self.requests.write().remove(&request_id) {
            if let Some(tx) = request.response_tx.take() {
                let _ = tx.send(true);
                return true;
            }
        }
        false
    }

    /// Denies a pending request.
    pub fn deny(&self, request_id: Uuid) -> bool {
        if let Some(mut request) = self.requests.write().remove(&request_id) {
            if let Some(tx) = request.response_tx.take() {
                let _ = tx.send(false);
                return true;
            }
        }
        false
    }

    /// Cancels a pending request.
    pub fn cancel(&self, request_id: Uuid) -> bool {
        self.requests.write().remove(&request_id).is_some()
    }

    /// Returns the number of pending requests.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.requests.read().len()
    }

    /// Lists pending request IDs.
    #[must_use]
    pub fn pending_requests(&self) -> Vec<Uuid> {
        self.requests.read().keys().copied().collect()
    }
}

impl std::fmt::Debug for ApprovalService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApprovalService")
            .field("pending_count", &self.pending_count())
            .finish()
    }
}

// Global singleton
static GLOBAL_SERVICE: RwLock<Option<Arc<ApprovalService>>> = RwLock::new(None);

/// Gets the global approval service.
pub fn get_approval_service() -> Arc<ApprovalService> {
    let read = GLOBAL_SERVICE.read();
    if let Some(ref service) = *read {
        return service.clone();
    }
    drop(read);

    let mut write = GLOBAL_SERVICE.write();
    if write.is_none() {
        *write = Some(Arc::new(ApprovalService::new()));
    }
    write.as_ref().unwrap().clone()
}

/// Clears the global approval service.
pub fn clear_approval_service() {
    *GLOBAL_SERVICE.write() = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_approval_approved() {
        let service = ApprovalService::new();
        let service = Arc::new(service);
        let service_clone = service.clone();

        let handle = tokio::spawn(async move {
            service_clone
                .request_approval("tool", "message", Duration::from_secs(5))
                .await
        });

        // Give time for request to be registered
        tokio::time::sleep(Duration::from_millis(10)).await;

        let requests = service.pending_requests();
        assert_eq!(requests.len(), 1);

        service.approve(requests[0]);

        let result = handle.await.unwrap();
        assert_eq!(result, Ok(true));
    }

    #[tokio::test]
    async fn test_approval_denied() {
        let service = Arc::new(ApprovalService::new());
        let service_clone = service.clone();

        let handle = tokio::spawn(async move {
            service_clone
                .request_approval("tool", "message", Duration::from_secs(5))
                .await
        });

        tokio::time::sleep(Duration::from_millis(10)).await;

        let requests = service.pending_requests();
        service.deny(requests[0]);

        let result = handle.await.unwrap();
        assert_eq!(result, Ok(false));
    }

    #[tokio::test]
    async fn test_approval_timeout() {
        let service = ApprovalService::new();

        let result = service
            .request_approval("tool", "message", Duration::from_millis(50))
            .await;

        assert_eq!(result, Err(ApprovalStatus::TimedOut));
    }
}
