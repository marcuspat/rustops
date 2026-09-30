// HNSW vector index over caller-supplied embeddings.
//
// Thin wrapper around the `hnsw_rs` crate: approximate nearest-neighbor
// search with cosine distance, plus a string-id ↔ internal-id mapping so
// callers can use their own identifiers.

use std::collections::HashMap;

use anyhow::Result;
use hnsw_rs::prelude::*;

/// HNSW indexer for vector search over caller-supplied embeddings.
pub struct HNSWIndexer {
    index: Hnsw<'static, f32, DistCosine>,
    dimensions: usize,
    /// caller id -> internal point id
    ids: HashMap<String, usize>,
    /// internal point id -> caller id
    rev: Vec<String>,
}

/// A single search hit.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    /// Cosine similarity in [-1, 1] (1.0 = identical direction).
    pub similarity: f32,
}

impl HNSWIndexer {
    /// Create a new index for vectors of the given dimensionality.
    pub fn new(dimensions: usize) -> Result<Self> {
        anyhow::ensure!(dimensions > 0, "dimensions must be > 0");
        // max connections per layer, capacity hint, max layers, ef_construction
        let index = Hnsw::new(16, 10_000, 16, 200, DistCosine {});
        Ok(Self {
            index,
            dimensions,
            ids: HashMap::new(),
            rev: Vec::new(),
        })
    }

    /// Index an embedding under a caller-supplied id.
    ///
    /// Note: HNSW graphs do not support deletion. Re-indexing an existing id
    /// points the id at the new vector, but the old vector stays in the
    /// graph (it can no longer be reported, only traversed).
    pub fn index(&mut self, id: &str, embedding: &[f32]) -> Result<()> {
        anyhow::ensure!(
            embedding.len() == self.dimensions,
            "embedding dimension mismatch: expected {}, got {}",
            self.dimensions,
            embedding.len()
        );

        let point_id = self.rev.len();
        self.index.insert((embedding, point_id));
        self.rev.push(id.to_string());
        self.ids.insert(id.to_string(), point_id);
        Ok(())
    }

    /// Search for up to `limit` entries with similarity >= `threshold`.
    ///
    /// This is an ANN index, not an exhaustive k-NN: the fetch window is an
    /// over-fetch, and entries below `threshold` inside the window consume
    /// slots without producing results — so a query can return fewer than
    /// `limit` entries even when more qualifying entries exist beyond the
    /// window. The entries that ARE returned are the nearest qualifying
    /// ones the graph traversal reached, ordered by similarity.
    ///
    /// Cost note: the fetch window grows by the superseded count on every
    /// query. When re-indexing dominates the workload, rebuild the index
    /// instead of re-indexing id by id.
    pub fn search(&self, query: &[f32], limit: usize, threshold: f32) -> Result<Vec<SearchResult>> {
        anyhow::ensure!(
            query.len() == self.dimensions,
            "query embedding dimension mismatch: expected {}, got {}",
            self.dimensions,
            query.len()
        );

        // Re-indexed ids leave superseded points in the graph (HNSW has no
        // deletion). Stale points can sit nearer the query than every live
        // one, so over-fetch by the superseded count. This makes it likely
        // the live entries fit in the window, not guaranteed: the HNSW
        // traversal is approximate and may return fewer points than asked.
        let superseded = self.rev.len().saturating_sub(self.ids.len());
        let fetch = limit.saturating_add(superseded).max(1);
        let ef_search = fetch * 4;
        let neighbours = self.index.search(query, fetch, ef_search);

        // Keep live points above the threshold, order by similarity
        // (descending), then cut at the limit: hnsw_rs does not guarantee
        // its results are distance-sorted, so sorting here is what makes
        // the truncation "nearest".
        let mut results: Vec<SearchResult> = neighbours
            .iter()
            .filter_map(|n| {
                let id = self.rev.get(n.d_id)?;
                // Skip points whose caller id has since been re-indexed to a
                // different internal point.
                if self.ids.get(id) != Some(&n.d_id) {
                    return None;
                }
                // DistCosine returns 1 - cosine_similarity.
                let similarity = 1.0 - n.distance;
                (similarity >= threshold).then(|| SearchResult {
                    id: id.clone(),
                    similarity,
                })
            })
            .collect();
        results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        Ok(results)
    }

    /// Index statistics.
    pub fn stats(&self) -> HNSWStats {
        HNSWStats {
            num_elements: self.index.get_nb_point(),
            live_elements: self.ids.len(),
            dimensions: self.dimensions,
        }
    }
}

/// Index statistics.
#[derive(Debug, Clone)]
pub struct HNSWStats {
    /// Total points in the HNSW graph, **including superseded ones** from
    /// re-indexed ids (HNSW has no deletion).
    pub num_elements: usize,
    /// Distinct live ids — what search can actually return.
    pub live_elements: usize,
    pub dimensions: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hnsw_index_and_search() {
        let mut indexer = HNSWIndexer::new(3).unwrap();

        indexer.index("doc1", &[1.0, 0.0, 0.0]).unwrap();
        indexer.index("doc2", &[0.0, 1.0, 0.0]).unwrap();
        indexer.index("doc3", &[0.9, 0.1, 0.0]).unwrap();

        let results = indexer.search(&[1.0, 0.0, 0.0], 3, 0.5).unwrap();

        assert!(!results.is_empty());
        assert_eq!(results[0].id, "doc1");
        assert!(results[0].similarity > 0.9);
        // Orthogonal doc2 must not pass the 0.5 similarity threshold.
        assert!(results.iter().all(|r| r.id != "doc2"));
    }

    #[test]
    fn test_reindexed_id_still_findable() {
        // Regression: re-indexing used to make an id unfindable when its
        // stale vector crowded the result window. A limit that spans the
        // whole index makes the assertion deterministic on every platform
        // (hnsw_rs entry points are OS-dependent; a limit-1 query on this
        // 4-point graph flaked on windows): fetch then covers every point,
        // so any correct traversal returns all ids — "a" must appear via
        // its LIVE vector, exactly once, with the live vector's similarity.
        let mut indexer = HNSWIndexer::new(3).unwrap();
        indexer.index("a", &[1.0, 0.0, 0.0]).unwrap();
        indexer.index("noise1", &[0.0, 1.0, 0.0]).unwrap();
        indexer.index("noise2", &[0.0, 0.0, 1.0]).unwrap();
        // Re-index "a" to a nearby but distinct direction.
        indexer.index("a", &[0.9, 0.1, 0.0]).unwrap();

        // Query the STALE direction with a whole-index limit: the stale
        // point is the exact nearest and gets filtered; "a" must still be
        // reported through its live vector.
        let stale_dir = indexer.search(&[1.0, 0.0, 0.0], 4, 0.0).unwrap();
        let a_hits: Vec<_> = stale_dir.iter().filter(|r| r.id == "a").collect();
        assert_eq!(
            a_hits.len(),
            1,
            "\"a\" must be reported exactly once via its live vector: {stale_dir:?}"
        );
        // The reported similarity must be the LIVE vector's cosine
        // (0.9 / sqrt(0.9² + 0.1²) ≈ 0.9939), never the stale vector's 1.0.
        let expected_live_similarity = 0.9f32 / (0.9f32 * 0.9 + 0.1f32 * 0.1).sqrt();
        assert!(
            (a_hits[0].similarity - expected_live_similarity).abs() < 1e-3,
            "reported similarity must be the live vector's ({expected_live_similarity:.4}), got {}",
            a_hits[0].similarity
        );

        // And the live direction still finds it trivially.
        let live = indexer.search(&[0.9, 0.1, 0.0], 1, 0.0).unwrap();
        assert!(
            live.iter().any(|r| r.id == "a"),
            "re-indexed id must be findable at its live direction: {live:?}"
        );
    }

    #[test]
    fn test_dimension_mismatch_rejected() {
        let mut indexer = HNSWIndexer::new(3).unwrap();
        assert!(indexer.index("bad", &[1.0, 0.0]).is_err());
        assert!(indexer.search(&[1.0, 0.0], 3, 0.0).is_err());
    }

    #[test]
    fn test_crate_cosine_distance_semantics() {
        let dist = DistCosine {};
        let a = [1.0f32, 0.0, 0.0];
        let b = [1.0f32, 0.0, 0.0];
        let c = [0.0f32, 1.0, 0.0];

        // Identical direction -> distance ~0; orthogonal -> distance ~1.
        assert!(dist.eval(&a, &b).abs() < 1e-6);
        assert!((dist.eval(&a, &c) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_stats() {
        let mut indexer = HNSWIndexer::new(2).unwrap();
        indexer.index("a", &[1.0, 0.0]).unwrap();
        indexer.index("b", &[0.0, 1.0]).unwrap();
        let stats = indexer.stats();
        assert_eq!(stats.num_elements, 2);
        assert_eq!(stats.live_elements, 2);
        assert_eq!(stats.dimensions, 2);
    }
}
