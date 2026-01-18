// Embedding model for vector generation
//
// Uses ONNX Runtime for efficient embedding generation

use anyhow::Result;
use ndarray::Array1;
use ort::{Environment, ExecutionProvider, Session, Value};
use tracing::{debug, info};

/// Embedding configuration
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    pub model_name: String,
    pub model_path: Option<String>,
    pub device: ort::ExecutionProviderDispatch,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model_name: "all-MiniLM-L6-v2".to_string(),
            model_path: None,
            device: ort::ExecutionProviderDispatch::CPU,
        }
    }
}

/// Embedding model
pub struct EmbeddingModel {
    session: Session,
    dimensions: usize,
}

impl EmbeddingModel {
    /// Create new embedding model
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        info!("Loading embedding model: {}", config.model_name);

        let environment = Environment::builder()
            .with_execution_providers([config.device])
            .build()?;

        // For now, use a simple placeholder
        // In production, download actual model
        let session = Session::builder(&environment)?
            .with_from_pretrained("sentence-transformers/all-MiniLM-L6-v2")
            .await?;

        let dimensions = 384; // all-MiniLM-L6-v2 produces 384-dim embeddings

        Ok(Self { session, dimensions })
    }

    /// Generate embedding for text
    pub async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        debug!("Generating embedding for text ({} chars)", text.len());

        // Tokenize (simplified)
        let tokens = self.tokenize(text);

        // Run inference
        let inputs = vec![
            Value::from_array(self.session.allocator(), ndarray::CowView::from(&tokens))?
        ];

        let outputs = self.session.run_async(inputs).await?;
        let embedding = outputs[0].try_extract::<Array2<f32>>()?;

        // Mean pooling
        let averaged = embedding.mean_axis(ndarray::Axis(1), 0)?;
        let result = averaged.to_vec();

        // Normalize
        let norm: f32 = result.iter().map(|x| x * x).sum::<f32>().sqrt();
        let normalized: Vec<f32> = result.iter().map(|x| x / norm).collect();

        Ok(normalized)
    }

    /// Get embedding dimensions
    pub fn dimensions(&self) -> usize {
        self.dimensions
    }

    /// Tokenize text (simplified)
    fn tokenize(&self, text: &str) -> ndarray::Array2<i64> {
        // Simplified tokenization
        // In production, use proper tokenizer
        let tokens: Vec<i64> = text
            .split_whitespace()
            .take(512)
            .map(|s| s.chars().take(4).count() as i64)
            .collect();

        // Pad to 512 tokens
        let mut padded = vec![0i64; 512];
        for (i, token) in tokens.iter().enumerate() {
            if i < 512 {
                padded[i] = *token;
            }
        }

        ndarray::Array2::from_shape_vec((1, 512), padded).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires model download
    async fn test_embedding_generation() {
        let config = EmbeddingConfig::default();
        let model = EmbeddingModel::new(config).await.unwrap();

        let embedding = model.embed("Hello, world!").await.unwrap();

        assert_eq!(embedding.len(), model.dimensions());

        // Check normalized
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }
}
