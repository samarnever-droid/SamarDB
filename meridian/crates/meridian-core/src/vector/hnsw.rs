//! Hierarchical Navigable Small World (HNSW) Multi-Layer Vector Graph Index.

use crate::vector::simd_dist::{cosine_similarity, euclidean_distance_sq};
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq)]
pub struct HnswNode {
    pub id: u64,
    pub vector: Vec<f32>,
    pub level: usize,
    pub neighbors: Vec<Vec<u64>>, // Neighbors per level: neighbors[l] = [u64]
}

#[derive(Debug, Clone, PartialEq)]
struct Candidate {
    id: u64,
    dist: f32,
}

impl Eq for Candidate {}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Reverse for min-heap
        other.dist.partial_cmp(&self.dist)
    }
}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}

pub struct HnswIndex {
    pub m: usize,                  // Max neighbors per layer
    pub ef_construction: usize,    // Beam size during build
    pub max_level: usize,
    pub entry_point: Option<u64>,
    pub nodes: HashMap<u64, HnswNode>,
}

impl HnswIndex {
    pub fn new(m: usize, ef_construction: usize) -> Self {
        Self {
            m: if m == 0 { 16 } else { m },
            ef_construction: if ef_construction == 0 { 64 } else { ef_construction },
            max_level: 0,
            entry_point: None,
            nodes: HashMap::new(),
        }
    }

    pub fn default_index() -> Self {
        Self::new(16, 64)
    }

    fn sample_level(&self) -> usize {
        let r: f64 = (self.nodes.len() as f64 + 1.0).fract();
        let ml = 1.0 / (self.m as f64).ln().max(1.0);
        ((-r.max(0.0001).ln()) * ml).floor() as usize
    }

    /// Inserts a vector embedding into the multi-layer HNSW graph.
    pub fn insert(&mut self, id: u64, vector: Vec<f32>) {
        let node_level = self.sample_level().min(4); // Cap max layer at 4 for bounded memory
        let mut neighbors = Vec::with_capacity(node_level + 1);
        for _ in 0..=node_level {
            neighbors.push(Vec::new());
        }

        let node = HnswNode {
            id,
            vector: vector.clone(),
            level: node_level,
            neighbors,
        };

        self.nodes.insert(id, node);

        if self.entry_point.is_none() {
            self.entry_point = Some(id);
            self.max_level = node_level;
            return;
        }

        let mut curr_ep = self.entry_point.unwrap();

        // 1. Top-down skip routing to insertion level
        for l in (node_level + 1..=self.max_level).rev() {
            curr_ep = self.greedy_search_layer(&vector, curr_ep, l);
        }

        // 2. Layer-by-layer link insertion
        for l in (0..=node_level.min(self.max_level)).rev() {
            let candidates = self.search_layer(&vector, curr_ep, self.ef_construction, l);
            let chosen_neighbors: Vec<u64> = candidates
                .into_iter()
                .filter(|(cid, _)| *cid != id)
                .take(self.m)
                .map(|(cid, _)| cid)
                .collect();

            // Link bidirectionally
            if let Some(target_node) = self.nodes.get_mut(&id) {
                target_node.neighbors[l] = chosen_neighbors.clone();
            }

            for &nbr in &chosen_neighbors {
                if let Some(nbr_node) = self.nodes.get_mut(&nbr) {
                    if l < nbr_node.neighbors.len() && !nbr_node.neighbors[l].contains(&id) {
                        nbr_node.neighbors[l].push(id);
                        if nbr_node.neighbors[l].len() > self.m * 2 {
                            nbr_node.neighbors[l].truncate(self.m * 2);
                        }
                    }
                }
            }

            if let Some(&first) = chosen_neighbors.first() {
                curr_ep = first;
            }
        }

        if node_level > self.max_level {
            self.max_level = node_level;
            self.entry_point = Some(id);
        }
    }

    /// Searches for Top-K approximate nearest neighbors via HNSW multi-layer routing.
    pub fn search(&self, query: &[f32], k: usize, ef_search: usize) -> Vec<(u64, f32)> {
        let entry_point = match self.entry_point {
            Some(ep) => ep,
            None => return Vec::new(),
        };

        let mut curr_ep = entry_point;

        // Top-down greedy jump through upper sparse layers
        for l in (1..=self.max_level).rev() {
            curr_ep = self.greedy_search_layer(query, curr_ep, l);
        }

        // Layer 0 beam search
        let candidates = self.search_layer(query, curr_ep, ef_search.max(k), 0);
        let mut results: Vec<(u64, f32)> = candidates.into_iter().take(k).collect();
        
        // Convert distance to cosine similarity
        for res in &mut results {
            if let Some(node) = self.nodes.get(&res.0) {
                res.1 = cosine_similarity(query, &node.vector);
            }
        }
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        results
    }

    fn greedy_search_layer(&self, query: &[f32], entry_node: u64, layer: usize) -> u64 {
        let mut curr = entry_node;
        let mut curr_dist = euclidean_distance_sq(query, &self.nodes[&curr].vector);

        loop {
            let mut changed = false;
            if let Some(node) = self.nodes.get(&curr) {
                if layer < node.neighbors.len() {
                    for &nbr in &node.neighbors[layer] {
                        if let Some(nbr_node) = self.nodes.get(&nbr) {
                            let d = euclidean_distance_sq(query, &nbr_node.vector);
                            if d < curr_dist {
                                curr_dist = d;
                                curr = nbr;
                                changed = true;
                            }
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }

        curr
    }

    fn search_layer(&self, query: &[f32], entry_node: u64, ef: usize, layer: usize) -> Vec<(u64, f32)> {
        let mut visited = HashSet::new();
        let mut candidates = BinaryHeap::new();
        let mut nearest = Vec::new();

        let initial_dist = euclidean_distance_sq(query, &self.nodes[&entry_node].vector);
        visited.insert(entry_node);
        candidates.push(Candidate { id: entry_node, dist: initial_dist });
        nearest.push((entry_node, initial_dist));

        while let Some(curr) = candidates.pop() {
            if let Some(node) = self.nodes.get(&curr.id) {
                if layer < node.neighbors.len() {
                    for &nbr in &node.neighbors[layer] {
                        if visited.insert(nbr) {
                            if let Some(nbr_node) = self.nodes.get(&nbr) {
                                let d = euclidean_distance_sq(query, &nbr_node.vector);
                                if nearest.len() < ef || d < nearest.last().map(|n| n.1).unwrap_or(f32::MAX) {
                                    candidates.push(Candidate { id: nbr, dist: d });
                                    nearest.push((nbr, d));
                                    nearest.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
                                    if nearest.len() > ef {
                                        nearest.pop();
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        nearest
    }

    pub fn count(&self) -> usize {
        self.nodes.len()
    }
}
