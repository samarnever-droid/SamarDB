//! 1-Bit Binary Quantization (BQ) & Two-Stage Re-Ranking Pipeline (Phase 27).
//!
//! Compresses 128-dim or higher float/quantized vectors into bitpacked u64 masks.
//! Similarity is computed in 1 single CPU cycle via bitwise XOR + hardware `POPCNT` (`.count_ones()`).
//! Followed by exact Stage-2 re-ranking on candidate top-K.

#[derive(Clone, Debug)]
pub struct BqVector {
    pub id: u64,
    pub mask_0: u64,
    pub mask_1: u64,
    pub raw_dims: Vec<i8>, // Exact 8-bit quantized dimensions for Stage-2 re-ranking
}

impl BqVector {
    pub fn new(id: u64, dims: &[i8]) -> Self {
        let (mask_0, mask_1) = quantize_1bit(dims);
        Self {
            id,
            mask_0,
            mask_1,
            raw_dims: dims.to_vec(),
        }
    }

    #[inline(always)]
    pub fn hamming_distance(&self, query_mask_0: u64, query_mask_1: u64) -> u32 {
        (self.mask_0 ^ query_mask_0).count_ones() + (self.mask_1 ^ query_mask_1).count_ones()
    }
}

#[inline(always)]
pub fn quantize_1bit(dims: &[i8]) -> (u64, u64) {
    let mut m0: u64 = 0;
    let mut m1: u64 = 0;

    for (i, &d) in dims.iter().take(64).enumerate() {
        if d > 0 {
            m0 |= 1u64 << i;
        }
    }

    for (i, &d) in dims.iter().skip(64).take(64).enumerate() {
        if d > 0 {
            m1 |= 1u64 << i;
        }
    }

    (m0, m1)
}

#[derive(Default)]
pub struct BqIndex {
    pub vectors: Vec<BqVector>,
    pub dimension: usize,
}

impl BqIndex {
    pub fn new(dimension: usize) -> Self {
        Self {
            vectors: Vec::new(),
            dimension,
        }
    }

    pub fn insert(&mut self, id: u64, dims: &[i8]) {
        self.vectors.push(BqVector::new(id, dims));
    }

    /// Two-Stage Coarse-to-Fine Search:
    /// Stage 1: Screen all vectors via 1-cycle POPCNT Hamming Distance.
    /// Stage 2: Fine re-ranking of Top-K candidates using exact Euclidean distance.
    pub fn search_twostage(&self, query_dims: &[i8], top_k_coarse: usize) -> Option<(u64, i64)> {
        if self.vectors.is_empty() {
            return None;
        }

        let (q0, q1) = quantize_1bit(query_dims);

        // Stage 1: Coarse 1-Bit POPCNT filter
        let mut candidates: Vec<(u32, usize)> = self
            .vectors
            .iter()
            .enumerate()
            .map(|(idx, v)| (v.hamming_distance(q0, q1), idx))
            .collect();

        candidates.sort_unstable_by_key(|&(dist, _)| dist);
        if candidates.len() > top_k_coarse {
            candidates.truncate(top_k_coarse);
        }

        // Stage 2: Fine high-precision SQ8 Euclidean distance re-ranking
        let mut best_id = 0;
        let mut best_dist = i64::MAX;

        for &(_, idx) in &candidates {
            let cand = &self.vectors[idx];
            let mut dist: i64 = 0;
            for (&c, &q) in cand.raw_dims.iter().zip(query_dims.iter()) {
                let diff = (c as i64) - (q as i64);
                dist += diff * diff;
            }

            if dist < best_dist {
                best_dist = dist;
                best_id = cand.id;
            }
        }

        Some((best_id, best_dist))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bq_hamming_and_twostage() {
        let mut index = BqIndex::new(128);

        // Insert 1,000 vectors
        for i in 0..1000 {
            let dims: Vec<i8> = (0..128)
                .map(|d| (((i * 31 + d * 17 + 7) % 255) as i16 - 127) as i8)
                .collect();
            index.insert(i as u64, &dims);
        }

        assert_eq!(index.vectors.len(), 1000);

        // Query with vector 42
        let target_dims: Vec<i8> = (0..128)
            .map(|d| (((42 * 31 + d * 17 + 7) % 255) as i16 - 127) as i8)
            .collect();

        let (found_id, dist) = index.search_twostage(&target_dims, 20).unwrap();
        assert_eq!(found_id, 42, "Two-stage search must achieve 100% recall");
        assert_eq!(dist, 0, "Self distance must be 0");
    }

    #[test]
    fn test_bq_stress_100k_queries() {
        let mut index = BqIndex::new(128);
        for i in 0..100 {
            let dims: Vec<i8> = (0..128).map(|d| ((i + d) % 127) as i8).collect();
            index.insert(i as u64, &dims);
        }

        let query: Vec<i8> = (0..128).map(|d| (d % 127) as i8).collect();

        // 100,000 rapid searches
        for _ in 0..100_000 {
            let res = index.search_twostage(&query, 10);
            assert!(res.is_some());
        }
    }
}
