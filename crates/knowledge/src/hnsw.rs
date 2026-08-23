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

    /// Search for the `limit` nearest entries with similarity >= `threshold`.
    pub fn search(&self, query: &[f32], limit: usize, threshold: f32) -> Result<Vec<SearchResult>> {
        anyhow::ensure!(
            query.len() == self.dimensions,
            "query embedding dimension mismatch: expected {}, got {}",
            self.dimensions,
            query.len()
        );

        let ef_search = (limit.max(1)) * 4;
        let neighbours = self.index.search(query, limit, ef_search);

        let mut results = Vec::with_capacity(neighbours.len());
        for n in neighbours {
            let Some(id) = self.rev.get(n.d_id) else {
                continue;
            };
            // Skip points whose caller id has since been re-indexed to a
            // different internal point.
            if self.ids.get(id) != Some(&n.d_id) {
                continue;
            }
            // DistCosine returns 1 - cosine_similarity.
            let similarity = 1.0 - n.distance;
            if similarity >= threshold {
                results.push(SearchResult {
                    id: id.clone(),
                    similarity,
                });
            }
        }
        Ok(results)
    }

    /// Index statistics.
    pub fn stats(&self) -> HNSWStats {
        HNSWStats {
            num_elements: self.index.get_nb_point(),
            dimensions: self.dimensions,
        }
    }
}

/// Index statistics.
#[derive(Debug, Clone)]
pub struct HNSWStats {
    pub num_elements: usize,
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
        assert_eq!(stats.dimensions, 2);
    }
}
