//! Mock Kafka cluster for testing.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock Kafka cluster for testing.
#[derive(Clone)]
pub struct MockKafka {
    topics: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
}

impl MockKafka {
    /// Create a new mock Kafka cluster.
    pub fn new() -> Self {
        Self {
            topics: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a topic.
    pub fn create_topic(&self, name: impl Into<String>) {
        let mut topics = self.topics.lock().unwrap();
        topics.entry(name.into()).or_default();
    }

    /// Produce a message to a topic.
    pub fn produce(&self, topic: impl Into<String>, message: Vec<u8>) {
        let mut topics = self.topics.lock().unwrap();
        topics.entry(topic.into()).or_default().push(message);
    }

    /// Consume messages from a topic.
    pub fn consume(&self, topic: impl Into<String>) -> Vec<Vec<u8>> {
        let topics = self.topics.lock().unwrap();
        topics.get(&topic.into()).cloned().unwrap_or_default()
    }

    /// Get the number of messages in a topic.
    pub fn len(&self, topic: impl Into<String>) -> usize {
        let topics = self.topics.lock().unwrap();
        topics.get(&topic.into()).map(|v| v.len()).unwrap_or(0)
    }

    /// Check if a topic is empty.
    pub fn is_empty(&self, topic: impl Into<String>) -> bool {
        self.len(topic) == 0
    }

    /// Clear all messages from a topic.
    pub fn clear(&self, topic: impl Into<String>) {
        let mut topics = self.topics.lock().unwrap();
        if let Some(messages) = topics.get_mut(&topic.into()) {
            messages.clear();
        }
    }
}

impl Default for MockKafka {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_kafka_creation() {
        let kafka = MockKafka::new();
        assert!(kafka.is_empty("test-topic"));
    }

    #[test]
    fn test_mock_kafka_create_topic() {
        let kafka = MockKafka::new();
        kafka.create_topic("test-topic");
        assert!(kafka.is_empty("test-topic"));
    }

    #[test]
    fn test_mock_kafka_produce_consume() {
        let kafka = MockKafka::new();
        kafka.create_topic("test-topic");

        kafka.produce("test-topic", b"message1".to_vec());
        kafka.produce("test-topic", b"message2".to_vec());

        assert_eq!(kafka.len("test-topic"), 2);

        let messages = kafka.consume("test-topic");
        assert_eq!(messages, vec![b"message1".to_vec(), b"message2".to_vec()]);
    }

    #[test]
    fn test_mock_kafka_clear() {
        let kafka = MockKafka::new();
        kafka.create_topic("test-topic");

        kafka.produce("test-topic", b"message1".to_vec());
        assert_eq!(kafka.len("test-topic"), 1);

        kafka.clear("test-topic");
        assert!(kafka.is_empty("test-topic"));
    }
}
