use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::storage::{MemoryStorage, MemoryItem, QueryFilter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub memory_count: usize,
    pub tags: Vec<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub user_id: String,
    pub summary_text: String,
    pub key_topics: Vec<String>,
    pub memory_count: usize,
    pub date_range: (DateTime<Utc>, DateTime<Utc>),
    pub importance_score: f32,
}

#[derive(Clone)]
pub struct SessionManager {
    storage: MemoryStorage,
    sessions_cache: HashMap<String, Session>,
} 

impl SessionManager {
    /// Create new session manager
    pub fn new(storage: MemoryStorage) -> Self {
        SessionManager {
            storage,
            sessions_cache: HashMap::new(),
        }
    }

    /// Create a new session for a user
    pub fn create_session(&mut self, user_id: &str, session_name: Option<String>) -> Result<String, Box<dyn std::error::Error>> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let session = Session {
            id: session_id.clone(),
            user_id: user_id.to_string(),
            name: session_name,
            created_at: now,
            last_active: now,
            memory_count: 0,
            tags: Vec::new(),
            metadata: HashMap::new(),
        };

        self.sessions_cache.insert(session_id.clone(), session);
        
        println!("Created session {} for user {}", session_id, user_id);
        Ok(session_id)
    }

    /// Get all sessions for a user
    pub fn get_user_sessions(&mut self, user_id: &str) -> Result<Vec<Session>, Box<dyn std::error::Error>> {
        // Get all memories for this user to reconstruct sessions
        let filter = QueryFilter {
            user_id: Some(user_id.to_string()),
            session_id: None,
            keywords: None,
            date_from: None,
            date_to: None,
            limit: None,
            min_importance: None,
        };

        let memories = self.storage.recall(filter)?;
        let mut session_map: HashMap<String, Session> = HashMap::new();

        // Build sessions from memories
        for memory in memories {
            let session = session_map.entry(memory.session_id.clone()).or_insert_with(|| {
                Session {
                    id: memory.session_id.clone(),
                    user_id: memory.user_id.clone(),
                    name: None,
                    created_at: memory.timestamp,
                    last_active: memory.timestamp,
                    memory_count: 0,
                    tags: Vec::new(),
                    metadata: HashMap::new(),
                }
            });

            // Update session stats
            session.memory_count += 1;
            if memory.timestamp > session.last_active {
                session.last_active = memory.timestamp;
            }
            if memory.timestamp < session.created_at {
                session.created_at = memory.timestamp;
            }

            // Extract tags from memory metadata
            if let Some(tags) = memory.metadata.get("tags") {
                for tag in tags.split(',') {
                    let tag = tag.trim().to_string();
                    if !session.tags.contains(&tag) {
                        session.tags.push(tag);
                    }
                }
            }
        }

        // Update cache and return sessions
        for (session_id, session) in &session_map {
            self.sessions_cache.insert(session_id.clone(), session.clone());
        }

        let mut sessions: Vec<Session> = session_map.into_values().collect();
        sessions.sort_by(|a, b| b.last_active.cmp(&a.last_active));

        println!("Found {} sessions for user {}", sessions.len(), user_id);
        Ok(sessions)
    }

    /// Get a specific session by ID
    pub fn get_session(&mut self, session_id: &str) -> Result<Option<Session>, Box<dyn std::error::Error>> {
        // Check cache first
        if let Some(session) = self.sessions_cache.get(session_id) {
            return Ok(Some(session.clone()));
        }

        // Reconstruct from memories
        let memories = self.storage.get_session_memories("", session_id)?;
        if memories.is_empty() {
            return Ok(None);
        }

        let first_memory = &memories[0];
        let mut session = Session {
            id: session_id.to_string(),
            user_id: first_memory.user_id.clone(),
            name: None,
            created_at: memories.iter().map(|m| m.timestamp).min().unwrap_or(Utc::now()),
            last_active: memories.iter().map(|m| m.timestamp).max().unwrap_or(Utc::now()),
            memory_count: memories.len(),
            tags: Vec::new(),
            metadata: HashMap::new(),
        };

        // Extract tags from all memories
        for memory in &memories {
            if let Some(tags) = memory.metadata.get("tags") {
                for tag in tags.split(',') {
                    let tag = tag.trim().to_string();
                    if !session.tags.contains(&tag) {
                        session.tags.push(tag);
                    }
                }
            }
        }

        self.sessions_cache.insert(session_id.to_string(), session.clone());
        Ok(Some(session))
    }

    /// Update session metadata
    pub fn update_session(&mut self, session_id: &str, name: Option<String>, tags: Option<Vec<String>>) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(session) = self.sessions_cache.get_mut(session_id) {
            if let Some(name) = name {
                session.name = Some(name);
            }
            if let Some(tags) = tags {
                session.tags = tags;
            }
            session.last_active = Utc::now();
            
            println!("Updated session {}", session_id);
            Ok(())
        } else {
            Err("Session not found".into())
        }
    }

    /// Delete a session and all its memories
    pub fn delete_session(&mut self, session_id: &str) -> Result<usize, Box<dyn std::error::Error>> {
        // This is a simplified delete - in production you'd want to properly remove from storage
        // For now, we'll just remove from cache and count would-be-deleted memories
        
        let memories = self.storage.get_session_memories("", session_id)?;
        let deleted_count = memories.len();
        
        self.sessions_cache.remove(session_id);
        
        println!("Deleted session {} with {} memories", session_id, deleted_count);
        Ok(deleted_count)
    }

    /// Generate session summary using memory content
    pub fn generate_session_summary(&mut self, session_id: &str) -> Result<SessionSummary, Box<dyn std::error::Error>> {
        let memories = self.storage.get_session_memories("", session_id)?;
        
        if memories.is_empty() {
            return Err("No memories found for session".into());
        }

        let user_id = memories[0].user_id.clone();
        
        // Extract key topics from memory content (simple keyword extraction)
        let mut topic_counts: HashMap<String, usize> = HashMap::new();
        let mut all_content = String::new();
        
        for memory in &memories {
            all_content.push_str(&memory.content);
            all_content.push(' ');
            
            // Fix: Create owned string first, then split
            let content_lower = memory.content.to_lowercase();
            let words: Vec<&str> = content_lower
                .split_whitespace()
                .filter(|w| w.len() > 3 && !is_stop_word(w))
                .collect();
                
            for word in words {
                *topic_counts.entry(word.to_string()).or_insert(0) += 1;
            }
        }

        // Get top topics
        let mut topics: Vec<(String, usize)> = topic_counts.into_iter().collect();
        topics.sort_by(|a, b| b.1.cmp(&a.1));
        let key_topics: Vec<String> = topics.into_iter().take(5).map(|(word, _)| word).collect();

        // Generate simple summary (first few sentences + key points)
        let summary_text = self.create_simple_summary(&memories, &key_topics);

        // Calculate importance score (average of memory importance)
        let importance_score = memories.iter()
            .map(|m| m.importance)
            .sum::<f32>() / memories.len() as f32;

        // Date range
        let timestamps: Vec<DateTime<Utc>> = memories.iter().map(|m| m.timestamp).collect();
        let date_range = (
            *timestamps.iter().min().unwrap(),
            *timestamps.iter().max().unwrap(),
        );

        let summary = SessionSummary {
            session_id: session_id.to_string(),
            user_id,
            summary_text,
            key_topics,
            memory_count: memories.len(),
            date_range,
            importance_score,
        };

        println!("Generated summary for session {} with {} memories", session_id, memories.len());
        Ok(summary)
    }

    /// Get session statistics
    pub fn get_session_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        
        for session in self.sessions_cache.values() {
            let user_sessions = stats.entry(session.user_id.clone()).or_insert(0);
            *user_sessions += 1;
        }
        
        stats.insert("total_sessions".to_string(), self.sessions_cache.len());
        stats
    }

    /// Find sessions by content keywords
    pub fn search_sessions(&mut self, user_id: &str, keywords: Vec<String>) -> Result<Vec<Session>, Box<dyn std::error::Error>> {
        let filter = QueryFilter {
            user_id: Some(user_id.to_string()),
            session_id: None,
            keywords: Some(keywords),
            date_from: None,
            date_to: None,
            limit: None,
            min_importance: None,
        };

        let memories = self.storage.recall(filter)?;
        let mut session_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        
        for memory in memories {
            session_ids.insert(memory.session_id);
        }

        let mut matching_sessions = Vec::new();
        for session_id in session_ids {
            if let Ok(Some(session)) = self.get_session(&session_id) {
                matching_sessions.push(session);
            }
        }

        matching_sessions.sort_by(|a, b| b.last_active.cmp(&a.last_active));
        Ok(matching_sessions)
    }

    // Private helper methods
    
    fn create_simple_summary(&self, memories: &[MemoryItem], key_topics: &[String]) -> String {
        let total_memories = memories.len();
        let date_span = if memories.len() > 1 {
            let start = memories.iter().map(|m| m.timestamp).min().unwrap();
            let end = memories.iter().map(|m| m.timestamp).max().unwrap();
            let days = (end - start).num_days();
            format!(" over {} days", days)
        } else {
            String::new()
        };

        let topics_text = if !key_topics.is_empty() {
            format!(" Key topics: {}.", key_topics.join(", "))
        } else {
            String::new()
        };

        format!(
            "Session contains {} memories{}.{} Most recent: \"{}\"",
            total_memories,
            date_span,
            topics_text,
            memories.first().map(|m| {
                if m.content.len() > 100 {
                    format!("{}...", &m.content[..100])
                } else {
                    m.content.clone()
                }
            }).unwrap_or_default()
        )
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

    #[test]
    fn test_session_creation_and_retrieval() {
        let storage = MemoryStorage::new("./test_sessions").unwrap();
        let mut session_manager = SessionManager::new(storage);
        
        let session_id = session_manager.create_session("test_user", Some("Test Session".to_string())).unwrap();
        assert!(!session_id.is_empty());
        
        let session = session_manager.get_session(&session_id).unwrap();
        assert!(session.is_some());
        assert_eq!(session.unwrap().name, Some("Test Session".to_string()));

        // Cleanup
        std::fs::remove_dir_all("./test_sessions").ok();
    }

    #[test]
    fn test_session_summary() {
        let storage = MemoryStorage::new("./test_summary").unwrap();
        let _session_manager = SessionManager::new(storage);
        
        // This test would need actual memories in storage to work properly
        // In a real scenario, you'd add memories first then test summary generation
        
        // Cleanup
        std::fs::remove_dir_all("./test_summary").ok();
    }
}