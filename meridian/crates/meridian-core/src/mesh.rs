//! Mesh (Phase 12): Consistent-Hash Topology & Popularity Gossip.

use std::collections::BTreeMap;
use std::sync::RwLock;

pub struct MeshCluster {
    ring: RwLock<BTreeMap<u64, String>>, // hash -> node_addr
    local_node: String,
}

impl MeshCluster {
    pub fn new(local_node: impl Into<String>) -> Self {
        let mut ring = BTreeMap::new();
        let local = local_node.into();
        let hash = crate::hash::hash_key(local.as_bytes());
        ring.insert(hash, local.clone());

        Self {
            ring: RwLock::new(ring),
            local_node: local,
        }
    }

    pub fn add_node(&self, node_addr: &str) {
        let hash = crate::hash::hash_key(node_addr.as_bytes());
        self.ring.write().unwrap().insert(hash, node_addr.to_string());
    }

    pub fn locate_node(&self, key: &[u8]) -> String {
        let hash = crate::hash::hash_key(key);
        let ring = self.ring.read().unwrap();
        if let Some((_, addr)) = ring.range(hash..).next() {
            addr.clone()
        } else if let Some((_, addr)) = ring.iter().next() {
            addr.clone()
        } else {
            self.local_node.clone()
        }
    }
}
