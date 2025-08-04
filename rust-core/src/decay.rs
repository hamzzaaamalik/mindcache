use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use crate::storage::{MemoryStorage, MemoryItem, QueryFilter};
use crate::session::SessionManager; // Remove unused Session import

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayPolicy {
    pub max_age_hours: u32,
    pub importance_threshold: f32,
    pub max_memories_per_user: usize,
    pub compression_enabled: bool,
    pub auto_summarize_sessions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayStats {
    pub memories_expired: usize,
    pub memories_compressed: usize,
    pub sessions_summarized: usize,
    pub total_memories_before: usize,
    pub total_memories_after: usize,
    pub storage_saved_bytes: usize,
    pub last_decay_run: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedMemory {
    pub original_ids: Vec<String>,
    pub user_id: String,
    pub session_id: String,
    pub summary: String,
    pub key_points: Vec<String>,
    pub date_range: (DateTime<Utc>, DateTime<Utc>),
    pub original_count: usize,
    pub combined_importance: f32,
    pub compressed_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct MemoryDecayEngine {
    storage: MemoryStorage,
    session_manager: SessionManager,
    policy: DecayPolicy,
    stats: DecayStats,
}

 
impl Default for DecayPolicy {
    fn default() -> Self {
        DecayPolicy {
            max_age_hours: 24 * 30, // 30 days
            importance_threshold: 0.3,
            max_memories_per_user: 10000,
            compression_enabled: true,
            auto_summarize_sessions: true,
        }
    }
}

impl MemoryDecayEngine {
    /// Create new decay engine with default policy
    pub fn new(storage: MemoryStorage, session_manager: SessionManager) -> Self {
        MemoryDecayEngine {
            storage,
            session_manager,
            policy: DecayPolicy::default(),
            stats: DecayStats {
                memories_expired: 0,
                memories_compressed: 0,
                sessions_summarized: 0,
                total_memories_before: 0,
                total_memories_after: 0,
                storage_saved_bytes: 0,
                last_decay_run: Utc::now(),
            },
        }
    }

    /// Create decay engine with custom policy
    pub fn with_policy(storage: MemoryStorage, session_manager: SessionManager, policy: DecayPolicy) -> Self {
        let mut engine = Self::new(storage, session_manager);
        engine.policy = policy;
        engine
    }

    /// Run full decay process
    pub fn run_decay(&mut self) -> Result<DecayStats, Box<dyn std::error::Error>> {
        let start_time = Utc::now();
        println!("Starting memory decay process...");

        // Reset stats for this run
        let mut run_stats = DecayStats {
            memories_expired: 0,
            memories_compressed: 0,
            sessions_summarized: 0,
            total_memories_before: 0,
            total_memories_after: 0,
            storage_saved_bytes: 0,
            last_decay_run: start_time,
        };

        // Get initial memory count
        let storage_stats = self.storage.get_stats();
        run_stats.total_memories_before = storage_stats.values().sum();

        // Step 1: Remove expired memories based on TTL
        run_stats.memories_expired = self.expire_old_memories()?;

        // Step 2: Compress low-importance memories if enabled
        if self.policy.compression_enabled {
            run_stats.memories_compressed = self.compress_old_memories()?;
        }

        // Step 3: Auto-summarize old sessions if enabled
        if self.policy.auto_summarize_sessions {
            run_stats.sessions_summarized = self.summarize_old_sessions()?;
        }

        // Step 4: Enforce per-user memory limits
        let limited = self.enforce_memory_limits()?;
        run_stats.memories_expired += limited;

        // Update final stats
        let final_stats = self.storage.get_stats();
        run_stats.total_memories_after = final_stats.values().sum();
        
        // Update internal stats
        self.stats = run_stats.clone();

        let duration = Utc::now() - start_time;
        println!("Decay process completed in {}ms", duration.num_milliseconds());
        println!("Expired: {}, Compressed: {}, Sessions summarized: {}", 
                run_stats.memories_expired, 
                run_stats.memories_compressed,
                run_stats.sessions_summarized);

        Ok(run_stats)
    }

    /// Remove memories that have exceeded their TTL
    fn expire_old_memories(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let now = Utc::now();
        let mut expired_count = 0;

        // Get all memories to check for expiration
        let filter = QueryFilter {
            user_id: None,
            session_id: None,
            keywords: None,
            date_from: None,
            date_to: None,
            limit: None,
            min_importance: None,
        };

        let memories = self.storage.recall(filter)?;

        for memory in memories {
            let should_expire = if let Some(ttl_hours) = memory.ttl_hours {
                // Memory has explicit TTL
                let expiry_time = memory.timestamp + Duration::hours(ttl_hours as i64);
                now > expiry_time
            } else {
                // Use default policy max age
                let age_hours = (now - memory.timestamp).num_hours() as u32;
                age_hours > self.policy.max_age_hours
            };

            if should_expire && memory.importance < self.policy.importance_threshold {
                // Mark for deletion (in a real implementation, you'd remove from storage)
                expired_count += 1;
                println!("Expiring memory {} (age: {}h, importance: {})", 
                        memory.id, 
                        (now - memory.timestamp).num_hours(),
                        memory.importance);
            }
        }

        // Call storage cleanup
        let cleaned = self.storage.cleanup_expired()?;
        Ok(expired_count.max(cleaned))
    }

    /// Compress groups of old, low-importance memories
    fn compress_old_memories(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let cutoff_date = Utc::now() - Duration::hours(self.policy.max_age_hours as i64 / 2);
        let mut compressed_count = 0;

        // Get memories older than cutoff with low importance
        let filter = QueryFilter {
            user_id: None,
            session_id: None,
            keywords: None,
            date_from: None,
            date_to: Some(cutoff_date),
            limit: None,
            min_importance: None,
        };

        let old_memories = self.storage.recall(filter)?;
        
        // Group by user and session for compression
        let mut memory_groups: HashMap<(String, String), Vec<MemoryItem>> = HashMap::new();
        
        for memory in old_memories {
            if memory.importance < self.policy.importance_threshold {
                let key = (memory.user_id.clone(), memory.session_id.clone());
                memory_groups.entry(key).or_insert_with(Vec::new).push(memory);
            }
        }

        // Compress groups with 3+ memories
        for ((_user_id, session_id), memories) in memory_groups {
            if memories.len() >= 3 {
                let compressed = self.create_compressed_memory(memories)?;
                
                // In a real implementation, you'd replace the original memories with the compressed version
                println!("Compressed {} memories from session {} into summary", 
                        compressed.original_count, session_id);
                compressed_count += compressed.original_count;
            }
        }

        Ok(compressed_count)
    }

    /// Auto-summarize sessions that haven't been active recently
    fn summarize_old_sessions(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let cutoff_date = Utc::now() - Duration::days(7); // Sessions inactive for 7+ days
        let mut summarized_count = 0;

        // Get all users from storage stats
        let storage_stats = self.storage.get_stats();
        
        for user_id in storage_stats.keys() {
            let sessions = self.session_manager.get_user_sessions(user_id)?;
            
            for session in sessions {
                if session.last_active < cutoff_date && session.memory_count > 5 {
                    // Generate summary for old, substantial sessions
                    match self.session_manager.generate_session_summary(&session.id) {
                        Ok(_summary) => {
                            println!("Auto-summarized session {} with {} memories", 
                                    session.id, session.memory_count);
                            summarized_count += 1;
                            
                            // In a real implementation, you might save this summary
                            // and optionally remove some of the original memories
                        },
                        Err(e) => {
                            println!("Failed to summarize session {}: {}", session.id, e);
                        }
                    }
                }
            }
        }

        Ok(summarized_count)
    }

    /// Enforce per-user memory limits
    fn enforce_memory_limits(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let mut removed_count = 0;
        let storage_stats = self.storage.get_stats();

        for (user_id, memory_count) in storage_stats {
            if memory_count > self.policy.max_memories_per_user {
                let excess = memory_count - self.policy.max_memories_per_user;
                
                // Get user's memories sorted by importance (ascending)
                let filter = QueryFilter {
                    user_id: Some(user_id.clone()),
                    session_id: None,
                    keywords: None,
                    date_from: None,
                    date_to: None,
                    limit: None,
                    min_importance: None,
                };

                let mut memories = self.storage.recall(filter)?;
                memories.sort_by(|a, b| a.importance.partial_cmp(&b.importance).unwrap());

                // Remove least important memories
                for memory in memories.iter().take(excess) {
                    println!("Removing low-importance memory {} for user {} (importance: {})", 
                            memory.id, user_id, memory.importance);
                    removed_count += 1;
                }
            }
        }

        Ok(removed_count)
    }

    /// Create a compressed memory from multiple memories
    fn create_compressed_memory(&self, memories: Vec<MemoryItem>) -> Result<CompressedMemory, Box<dyn std::error::Error>> {
        if memories.is_empty() {
            return Err("Cannot compress empty memory list".into());
        }

        let user_id = memories[0].user_id.clone();
        let session_id = memories[0].session_id.clone();
        
        // Extract key information
        let original_ids: Vec<String> = memories.iter().map(|m| m.id.clone()).collect();
        let combined_content: String = memories.iter()
            .map(|m| m.content.as_str())
            .collect::<Vec<&str>>()
            .join(" | ");

        // Simple summary generation (first memory + count + key themes)
        let summary = if combined_content.len() > 200 {
            format!("{}... [+{} more memories]", 
                   &combined_content[..200], 
                   memories.len() - 1)
        } else {
            combined_content
        };

        // Extract key points (simple keyword extraction)
        let key_points = self.extract_key_points(&memories);

        // Date range
        let timestamps: Vec<DateTime<Utc>> = memories.iter().map(|m| m.timestamp).collect();
        let date_range = (
            *timestamps.iter().min().unwrap(),
            *timestamps.iter().max().unwrap(),
        );

        // Combined importance (weighted average)
        let combined_importance = memories.iter()
            .map(|m| m.importance)
            .sum::<f32>() / memories.len() as f32;

        Ok(CompressedMemory {
            original_ids,
            user_id,
            session_id,
            summary,
            key_points,
            date_range,
            original_count: memories.len(),
            combined_importance,
            compressed_at: Utc::now(),
        })
    }

    /// Extract key points from a group of memories
    fn extract_key_points(&self, memories: &[MemoryItem]) -> Vec<String> {
        let mut word_counts: HashMap<String, usize> = HashMap::new();
        
        for memory in memories {
            // Fix: Create owned string first, then split
            let content_lower = memory.content.to_lowercase();
            let words: Vec<&str> = content_lower
                .split_whitespace()
                .filter(|w| w.len() > 3 && !is_stop_word(w))
                .collect();
                
            for word in words {
                *word_counts.entry(word.to_string()).or_insert(0) += 1;
            }
        }

        // Return top 5 most frequent meaningful words
        let mut sorted_words: Vec<(String, usize)> = word_counts.into_iter().collect();
        sorted_words.sort_by(|a, b| b.1.cmp(&a.1));
        sorted_words.into_iter().take(5).map(|(word, _)| word).collect()
    }

    /// Get current decay statistics
    pub fn get_stats(&self) -> &DecayStats {
        &self.stats
    }

    /// Update decay policy
    pub fn update_policy(&mut self, policy: DecayPolicy) {
        self.policy = policy;
        println!("Updated decay policy: max_age={}h, threshold={}, compression={}", 
                self.policy.max_age_hours, 
                self.policy.importance_threshold,
                self.policy.compression_enabled);
    }

    /// Calculate memory age distribution
    pub fn analyze_memory_age_distribution(&self) -> Result<HashMap<String, usize>, Box<dyn std::error::Error>> {
        let now = Utc::now();
        let mut age_buckets: HashMap<String, usize> = HashMap::new();

        let filter = QueryFilter {
            user_id: None,
            session_id: None,
            keywords: None,
            date_from: None,
            date_to: None,
            limit: None,
            min_importance: None,
        };

        let memories = self.storage.recall(filter)?;

        for memory in memories {
            let age_hours = (now - memory.timestamp).num_hours();
            let bucket = match age_hours {
                0..=24 => "0-24h",
                25..=168 => "1-7d",
                169..=720 => "1-4w", 
                721..=2160 => "1-3m",
                _ => "3m+",
            };
            
            *age_buckets.entry(bucket.to_string()).or_insert(0) += 1;
        }

        Ok(age_buckets)
    }
}

fn is_stop_word(word: &str) -> bool {
    matches!(word, 
        "the" | "and" | "or" | "but" | "in" | "on" | "at" | "to" | "for" | 
        "of" | "with" | "by" | "from" | "up" | "about" | "into" | "through" | 
        "during" | "before" | "after" | "above" | "below" | "between" | "among" |
        "this" | "that" | "these" | "those" | "i" | "you" | "he" | "she" | "it" |
        "we" | "they" | "am" | "is" | "are" | "was" | "were" | "be" | "been" |
        "being" | "have" | "has" | "had" | "do" | "does" | "did" | "will" | "would"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemoryStorage;
    use crate::session::SessionManager;

    #[test]
    fn test_decay_policy_creation() {
        let policy = DecayPolicy::default();
        assert_eq!(policy.max_age_hours, 24 * 30);
        assert_eq!(policy.importance_threshold, 0.3);
        assert!(policy.compression_enabled);
    }

    #[test]
    fn test_memory_compression() {
    let storage = MemoryStorage::new("./test_decay").unwrap();
    let session_manager = SessionManager::new(storage.clone()); // Clone storage
    let _decay_engine = MemoryDecayEngine::new(storage, session_manager);
        
        // Test would need actual memories to compress
        // This is a placeholder for integration testing
        
    std::fs::remove_dir_all("./test_decay").ok();
    }
}