use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write, Seek, SeekFrom};
use std::path::Path;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub user_id: String,
    pub session_id: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
    pub ttl_hours: Option<u32>,
    pub importance: f32, // 0.0 to 1.0 for decay prioritization
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFilter {
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub date_from: Option<DateTime<Utc>>,
    pub date_to: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
    pub min_importance: Option<f32>,
}

pub struct MemoryStorage {
    storage_path: String,
    index_path: String,
    memory_index: HashMap<String, Vec<usize>>, // user_id -> file positions
}

impl Clone for MemoryStorage {
    fn clone(&self) -> Self {
        // Create a new storage instance with the same paths
        // This is a simplified clone - in production you might want to share the index
        MemoryStorage {
            storage_path: self.storage_path.clone(),
            index_path: self.index_path.clone(),
            memory_index: self.memory_index.clone(),
        }
    }
}

impl MemoryStorage {
    /// Create new storage instance with specified directory
    pub fn new(storage_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(storage_dir)?;
        
        let storage_path = format!("{}/memories.bin", storage_dir);
        let index_path = format!("{}/index.bin", storage_dir);
        
        let mut storage = MemoryStorage {
            storage_path,
            index_path,
            memory_index: HashMap::new(),
        };
        
        // Load existing index if available
        storage.load_index()?;
        
        Ok(storage)
    }

    /// Save a memory item to persistent storage
    pub fn save(&mut self, memory: MemoryItem) -> Result<String, Box<dyn std::error::Error>> {
        // Generate ID if not provided
        let memory_id = if memory.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            memory.id.clone()
        };

        let mut memory_with_id = memory;
        memory_with_id.id = memory_id.clone();

        // Serialize memory item
        let serialized = bincode::serialize(&memory_with_id)?;
        
        // Open file for appending
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.storage_path)?;
        
        // Get current position before writing
        let position = file.seek(SeekFrom::End(0))?;
        
        // Write length prefix + data
        let len = serialized.len() as u32;
        file.write_all(&len.to_le_bytes())?;
        file.write_all(&serialized)?;
        file.flush()?;
        
        // Update index
        self.memory_index
            .entry(memory_with_id.user_id.clone())
            .or_insert_with(Vec::new)
            .push(position as usize);
        
        // Persist index
        self.save_index()?;
        
        println!("Memory saved: {} for user {}", memory_id, memory_with_id.user_id);
        Ok(memory_id)
    }

    /// Recall memories based on query filters
    pub fn recall(&self, filter: QueryFilter) -> Result<Vec<MemoryItem>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        
        // If user_id specified, only search that user's memories
        let user_ids: Vec<String> = if let Some(user_id) = &filter.user_id {
            vec![user_id.clone()]
        } else {
            self.memory_index.keys().cloned().collect()
        };

        for user_id in user_ids {
            if let Some(positions) = self.memory_index.get(&user_id) {
                for &position in positions {
                    if let Ok(memory) = self.read_memory_at_position(position) {
                        if self.matches_filter(&memory, &filter) {
                            results.push(memory);
                        }
                    }
                }
            }
        }

        // Sort by timestamp (newest first)
        results.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        
        // Apply limit
        if let Some(limit) = filter.limit {
            results.truncate(limit);
        }

        println!("Recalled {} memories", results.len());
        Ok(results)
    }

    /// Get all memories for a specific session
    pub fn get_session_memories(&self, user_id: &str, session_id: &str) -> Result<Vec<MemoryItem>, Box<dyn std::error::Error>> {
        let filter = QueryFilter {
            user_id: Some(user_id.to_string()),
            session_id: Some(session_id.to_string()),
            keywords: None,
            date_from: None,
            date_to: None,
            limit: None,
            min_importance: None,
        };
        
        self.recall(filter)
    }

    /// Get memory statistics
    pub fn get_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        
        for (user_id, positions) in &self.memory_index {
            stats.insert(user_id.clone(), positions.len());
        }
        
        stats
    }

    /// Clean up expired memories (called by decay system)
    pub fn cleanup_expired(&mut self) -> Result<usize, Box<dyn std::error::Error>> {
        let now = Utc::now();
        let mut removed_count = 0;

        // This is a simplified cleanup - in production, you'd want to rebuild the file
        // For now, we'll mark expired items by updating their importance to 0
        for user_id in self.memory_index.keys().cloned().collect::<Vec<_>>() {
            if let Some(positions) = self.memory_index.get(&user_id).cloned() {
                for position in positions {
                    if let Ok(memory) = self.read_memory_at_position(position) {
                        if let Some(ttl_hours) = memory.ttl_hours {
                            let expiry = memory.timestamp + chrono::Duration::hours(ttl_hours as i64);
                            if now > expiry {
                                removed_count += 1;
                                // In a real implementation, mark for deletion
                            }
                        }
                    }
                }
            }
        }

        println!("Cleaned up {} expired memories", removed_count);
        Ok(removed_count)
    }

    // Private helper methods

    fn matches_filter(&self, memory: &MemoryItem, filter: &QueryFilter) -> bool {
        // User ID filter
        if let Some(ref user_id) = filter.user_id {
            if memory.user_id != *user_id {
                return false;
            }
        }

        // Session ID filter
        if let Some(ref session_id) = filter.session_id {
            if memory.session_id != *session_id {
                return false;
            }
        }

        // Date range filter
        if let Some(date_from) = filter.date_from {
            if memory.timestamp < date_from {
                return false;
            }
        }

        if let Some(date_to) = filter.date_to {
            if memory.timestamp > date_to {
                return false;
            }
        }

        // Importance filter
        if let Some(min_importance) = filter.min_importance {
            if memory.importance < min_importance {
                return false;
            }
        }

        // Keyword filter (simple text search)
        if let Some(ref keywords) = filter.keywords {
            let content_lower = memory.content.to_lowercase();
            let found = keywords.iter().any(|keyword| {
                content_lower.contains(&keyword.to_lowercase())
            });
            if !found {
                return false;
            }
        }

        true
    }

    fn read_memory_at_position(&self, position: usize) -> Result<MemoryItem, Box<dyn std::error::Error>> {
        let mut file = File::open(&self.storage_path)?;
        file.seek(SeekFrom::Start(position as u64))?;
        
        // Read length prefix
        let mut len_bytes = [0u8; 4];
        std::io::Read::read_exact(&mut file, &mut len_bytes)?;
        let len = u32::from_le_bytes(len_bytes);
        
        // Read data
        let mut data = vec![0u8; len as usize];
        std::io::Read::read_exact(&mut file, &mut data)?;
        
        // Deserialize
        let memory: MemoryItem = bincode::deserialize(&data)?;
        Ok(memory)
    }

    fn load_index(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if Path::new(&self.index_path).exists() {
            let file = File::open(&self.index_path)?;
            let reader = BufReader::new(file);
            
            for line in reader.lines() {
                let line = line?;
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() == 2 {
                    let user_id = parts[0].to_string();
                    let positions: Result<Vec<usize>, _> = parts[1]
                        .split(',')
                        .filter(|s| !s.is_empty())
                        .map(|s| s.parse())
                        .collect();
                    
                    if let Ok(positions) = positions {
                        self.memory_index.insert(user_id, positions);
                    }
                }
            }
        }
        Ok(())
    }

    fn save_index(&self) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::create(&self.index_path)?;
        let mut writer = BufWriter::new(file);
        
        for (user_id, positions) in &self.memory_index {
            let positions_str: Vec<String> = positions.iter().map(|p| p.to_string()).collect();
            writeln!(writer, "{}:{}", user_id, positions_str.join(","))?;
        }
        
        writer.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_save_and_recall() {
        let mut storage = MemoryStorage::new("./test_storage").unwrap();
        
        let memory = MemoryItem {
            id: "".to_string(),
            user_id: "test_user".to_string(),
            session_id: "session_1".to_string(),
            content: "I love trading gold futures".to_string(),
            metadata: HashMap::new(),
            timestamp: Utc::now(),
            ttl_hours: Some(24),
            importance: 0.8,
        };

        let memory_id = storage.save(memory).unwrap();
        assert!(!memory_id.is_empty());

        let filter = QueryFilter {
            user_id: Some("test_user".to_string()),
            session_id: None,
            keywords: Some(vec!["gold".to_string()]),
            date_from: None,
            date_to: None,
            limit: Some(10),
            min_importance: None,
        };

        let results = storage.recall(filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "I love trading gold futures");

        // Cleanup
        std::fs::remove_dir_all("./test_storage").ok();
    }
}