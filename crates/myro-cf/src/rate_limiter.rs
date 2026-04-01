use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, Instant};

/// Rate limiter enforcing 1 request per 2 seconds for the CF API.
pub struct RateLimiter {
    last_request: Arc<Mutex<Option<Instant>>>,
    min_interval: Duration,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            last_request: Arc::new(Mutex::new(None)),
            min_interval: Duration::from_secs(2),
        }
    }

    /// Wait until we're allowed to make the next request.
    pub async fn wait(&self) {
        let mut last = self.last_request.lock().await;
        if let Some(prev) = *last {
            let elapsed = prev.elapsed();
            if elapsed < self.min_interval {
                tokio::time::sleep(self.min_interval - elapsed).await;
            }
        }
        *last = Some(Instant::now());
    }
}
