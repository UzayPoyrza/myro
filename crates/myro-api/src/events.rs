use anyhow::Result;
use std::sync::Mutex;
use std::time::Instant;

use crate::client::SupabaseClient;
use crate::types::EventRow;

const FLUSH_INTERVAL_SECS: u64 = 30;
const MAX_BATCH_SIZE: usize = 100;

/// Buffered event tracker. Collects events and flushes periodically or on demand.
pub struct EventBatch {
    client: SupabaseClient,
    buffer: Mutex<Vec<EventRow>>,
    last_flush: Mutex<Instant>,
}

impl EventBatch {
    pub fn new(client: SupabaseClient) -> Self {
        Self {
            client,
            buffer: Mutex::new(Vec::new()),
            last_flush: Mutex::new(Instant::now()),
        }
    }

    /// Queue an event. Auto-flushes if buffer is full or interval elapsed.
    pub fn track(&self, event_type: &str, payload: serde_json::Value) {
        let event = EventRow {
            user_id: self.client.user_id.clone(),
            event_type: event_type.to_string(),
            payload: Some(payload),
        };

        let should_flush = {
            let mut buf = self.buffer.lock().unwrap();
            buf.push(event);
            let flush = self.last_flush.lock().unwrap();
            buf.len() >= MAX_BATCH_SIZE || flush.elapsed().as_secs() >= FLUSH_INTERVAL_SECS
        };

        if should_flush {
            let _ = self.flush();
        }
    }

    /// Flush all buffered events to Supabase. Best-effort: errors are logged but not propagated.
    pub fn flush(&self) -> Result<()> {
        let events: Vec<EventRow> = {
            let mut buf = self.buffer.lock().unwrap();
            let mut flush = self.last_flush.lock().unwrap();
            *flush = Instant::now();
            std::mem::take(&mut *buf)
        };

        if events.is_empty() {
            return Ok(());
        }

        self.client.post("events", &events, false)
    }
}

impl Drop for EventBatch {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}
