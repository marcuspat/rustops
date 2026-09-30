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
        let raw: Vec<(usize, f32)> = neighbours
            .iter()
            .map(|n| (n.d_id, n.distance))
            .collect();

        Ok(live_results(&raw, &self.rev, &self.ids, limit, threshold))
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

/// Filter raw ANN neighbour hits down to live results.
///
/// Pure function — extracted so the re-index regression coverage does not
/// depend on hnsw_rs's traversal, which is randomised **per process**
/// (in-process retries are correlated and cannot stabilise a graph query).
/// Semantics:
/// - a hit whose id has been re-indexed to a different point is dropped
///   (the superseded/stale point);
/// - a hit below `threshold` similarity (DistCosine: `1 - distance`) is
///   dropped;
/// - survivors are ordered by similarity descending (hnsw_rs does not
///   guarantee distance order — the sort is what makes truncation
///   "nearest") and cut at `limit`.
fn live_results(
    neighbours: &[(usize, f32)],
    rev: &[String],
    ids: &HashMap<String, usize>,
    limit: usize,
    threshold: f32,
) -> Vec<SearchResult> {
    let mut results: Vec<SearchResult> = neighbours
        .iter()
        .filter_map(|&(d_id, distance)| {
            let id = rev.get(d_id)?;
            if ids.get(id) != Some(&d_id) {
                return None;
            }
            let similarity = 1.0 - distance;
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
    results
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
    fn test_reindex_regression_filter_drops_stale_keeps_live() {
        // The re-index regression, tested where it is deterministic: the
        // filter. "a" was indexed at point 0, then re-indexed at point 3 —
        // point 0 is now stale. The ANN returns the stale point as the
        // exact nearest of the query; the filter must drop it and keep the
        // live point, even at limit 1. (hnsw_rs's traversal is randomised
        // per process, so this lives in the pure function, not a graph
        // query — measured: some processes never surface the live point at
        // limit 1 no matter how many in-process retries.)
        let mut rev = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        rev.push("a".to_string()); // re-indexed point
        let mut ids = HashMap::new();
        ids.insert("a".to_string(), 3);
        ids.insert("b".to_string(), 1);
        ids.insert("c".to_string(), 2);

        // Traversal returns stale "a" (distance 0.0), live "a" (distance
        // 0.006), then the others.
        let neighbours = vec![(0usize, 0.0f32), (3, 0.006), (1, 1.0), (2, 1.0)];
        let results = live_results(&neighbours, &rev, &ids, 1, 0.0);

        assert_eq!(results.len(), 1, "live 'a' survives at limit 1");
        assert_eq!(results[0].id, "a");
        assert!(
            results[0].similarity < 1.0 - 1e-3,
            "reported 'a' must be the live vector (~0.994), never the stale 1.0"
        );
    }

    #[test]
    fn test_live_results_orders_thresholds_and_truncates() {
        let rev: Vec<String> = ["x", "y", "z"].map(String::from).to_vec();
        let ids: HashMap<String, usize> =
            [("x", 0usize), ("y", 1), ("z", 2)]
                .map(|(k, v)| (k.to_string(), v))
                .into();

        // Distances: x=0.1 (sim 0.9), y=0.5 (sim 0.5), z=0.9 (sim 0.1).
        // Unsorted input, z below the 0.2 threshold: output must be
        // similarity-descending with z dropped.
        let neighbours = vec![(1usize, 0.5f32), (0, 0.1), (2, 0.9)];
        let results = live_results(&neighbours, &rev, &ids, 3, 0.2);
        let ids_out: Vec<&str> = results.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids_out, vec!["x", "y"], "similarity-descending, z below threshold");
        assert!((results[0].similarity - 0.9).abs() < 1e-6);

        // Truncation keeps the most similar.
        let truncated = live_results(&neighbours, &rev, &ids, 1, 0.0);
        assert_eq!(truncated.len(), 1);
        assert_eq!(truncated[0].id, "x");
    }

    #[test]
    fn test_reindex_bookkeeping_moves_the_id() {
        // What the graph-level API guarantees deterministically: the id
        // remaps to a new point and the old point stays in the graph
        // (HNSW has no deletion), i.e. the over-fetch input grows.
        let mut indexer = HNSWIndexer::new(3).unwrap();
        indexer.index("a", &[1.0, 0.0, 0.0]).unwrap();
        indexer.index("noise1", &[0.0, 1.0, 0.0]).unwrap();
        indexer.index("noise2", &[0.0, 0.0, 1.0]).unwrap();
        let old_point = *indexer.ids.get("a").expect("id mapped");
        indexer.index("a", &[0.9, 0.1, 0.0]).unwrap();
        let new_point = *indexer.ids.get("a").expect("id mapped");
        assert_ne!(old_point, new_point, "re-index must move the id's point");
        assert!(
            indexer.rev.len() > indexer.ids.len(),
            "the superseded point stays in the graph"
        );

        // The live direction finds the id through the real graph: the
        // live point is the exact nearest neighbour of this query, which
        // greedy descent reliably reaches.
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
