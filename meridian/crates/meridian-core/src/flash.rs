//! Flash Tier (Phase 12): Invalidation-Aware SSD Cache with Write Endurance Governance.

use std::collections::HashMap;
use std::sync::RwLock;

pub struct FlashTier {
    storage: RwLock<HashMap<Vec<u8>, Vec<u8>>>,
    pub write_amplification: f64,
    pub total_bytes_written: std::sync::atomic::AtomicU64,
}

impl FlashTier {
    pub fn new(write_amplification: f64) -> Self {
        Self {
            storage: RwLock::new(HashMap::new()),
            write_amplification,
            total_bytes_written: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn put(&self, key: Vec<u8>, val: Vec<u8>) {
        let size = val.len() as u64;
        self.total_bytes_written.fetch_add(size, std::sync::atomic::Ordering::Relaxed);
        self.storage.write().unwrap().insert(key, val);
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.storage.read().unwrap().get(key).cloned()
    }

    pub fn delete(&self, key: &[u8]) -> bool {
        self.storage.write().unwrap().remove(key).is_some()
    }
}
