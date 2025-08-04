//! Advanced session management example
//! 
//! Demonstrates:
//! - Creating and organizing sessions
//! - Session metadata and tagging
//! - Cross-session memory search
//! - Session analytics and insights

use mindcache_core::{MindCache, MindCacheConfig, QueryFilter};
use std::collections::HashMap;
use chrono::Utc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("📁 MindCache Session Management Example");
    println!("=======================================\n");

    let mut cache = MindCache::with_config(MindCacheConfig {
        storage_path: "./session_example_data".to_string(),
        ..Default::default()
    })?;

    let user_id = "session_demo_user";
    
    // Create multiple themed sessions
    println!("🆕 Creating themed sessions...\n");

    let trading_session = cache.create_session(user_id, Some("Day Trading Journal"))?;
    let research_session = cache.create_session(user_id, Some("Market Research"))?;
    let learning_session = cache.create_session(user_id, Some("Investment Learning"))?;
    let personal_session = cache.create_session(user_id, Some("Personal Finance"))?;

    println!("✅ Created 4 themed sessions\n");

    // Populate sessions with realistic content
    println!("💾 Adding realistic trading memories...\n");

    // Day Trading Journal entries
    let trading_memories = vec![
        ("Opened AAPL position at $175.50. Stop loss at $170. Target $185.", "entry", 0.8),
        ("AAPL hit first target. Took partial profits. Moving stop to breakeven.", "management", 0.9),
        ("Closed AAPL position at $184.20. +$8.70 per share. Good discipline.", "exit", 0.7),
        ("TSLA breaking out of consolidation. Watching for volume confirmation.", "watchlist", 0.6),
        ("Market showing weakness near close. Staying flat into tomorrow.", "market-analysis", 0.5),
    ];

    for (content, tag, importance) in trading_memories {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), tag.to_string());
        metadata.insert("session_type".to_string(), "trading".to_string());
        
        cache.save_with_options(user_id, &trading_session, content, Some(metadata), importance, None)?;
    }

    // Market Research entries
    let research_memories = vec![
        ("Fed meeting minutes suggest dovish pivot. Rate cuts possible by Q2.", "fed", 0.9),
        ("Semiconductor industry showing signs of recovery. NVDA earnings key.", "sector", 0.8),
        ("Oil inventory data bearish. WTI could test $70 support.", "commodities", 0.7),
        ("Consumer spending data mixed. Retail stocks under pressure.", "economic", 0.6),
        ("Geopolitical tensions rising. Safe havens getting bid.", "geopolitical", 0.8),
    ];

    for (content, tag, importance) in research_memories {
        let mut metadata = HashMap::new();
        metadata.insert("research_type".to_string(), tag.to_string());
        metadata.insert("session_type".to_string(), "research".to_string());
        
        cache.save_with_options(user_id, &research_session, content, Some(metadata), importance, None)?;
    }

    // Investment Learning entries
    let learning_memories = vec![
        ("Read about momentum investing. Key is buying strength, selling weakness.", "strategy", 0.7),
        ("Risk management rule: Never risk more than 2% per trade.", "risk", 0.9),
        ("Position sizing formula: (Account Size × Risk %) ÷ (Entry - Stop)", "formula", 0.8),
        ("Market cycles: Accumulation → Markup → Distribution → Markdown", "theory", 0.6),
        ("Learned about sector rotation. Technology leads in growth phases.", "sectors", 0.5),
    ];

    for (content, tag, importance) in learning_memories {
        let mut metadata = HashMap::new();
        metadata.insert("learning_type".to_string(), tag.to_string());
        metadata.insert("session_type".to_string(), "education".to_string());
        
        cache.save_with_options(user_id, &learning_session, content, Some(metadata), importance, None)?;
    }

    // Personal Finance entries
    let personal_memories = vec![
        ("Emergency fund goal: $50k. Currently at $32k. Need $18k more.", "emergency", 0.8),
        ("401k contribution increased to 15%. Company matches 5%.", "retirement", 0.7),
        ("Mortgage rate locked at 6.5%. Considering refinance if rates drop.", "mortgage", 0.6),
        ("Tax loss harvesting opportunity in December. Review positions.", "taxes", 0.9),
        ("Insurance review due. Life insurance needs update after promotion.", "insurance", 0.5),
    ];

    for (content, tag, importance) in personal_memories {
        let mut metadata = HashMap::new();
        metadata.insert("personal_type".to_string(), tag.to_string());
        metadata.insert("session_type".to_string(), "personal".to_string());
        
        cache.save_with_options(user_id, &personal_session, content, Some(metadata), importance, None)?;
    }

    println!("✅ Added 20 memories across 4 sessions\n");

    // Demonstrate session analytics
    println!("📊 Session Analytics...\n");

    let sessions = cache.get_user_sessions(user_id)?;
    println!("📁 All Sessions Overview:");
    for session in &sessions {
        println!("   • {} ({} memories)", 
                session.name.as_ref().unwrap_or(&"Unnamed".to_string()),
                session.memory_count);
        println!("     Last active: {}", session.last_active.format("%Y-%m-%d %H:%M"));
        println!("     Tags: {}", session.tags.join(", "));
        println!();
    }

    // Generate detailed session summaries
    println!("📋 Detailed Session Summaries...\n");

    let session_map = vec![
        (&trading_session, "Day Trading Journal"),
        (&research_session, "Market Research"),
        (&learning_session, "Investment Learning"),
        (&personal_session, "Personal Finance"),
    ];

    for (session_id, session_name) in session_map {
        println!("📁 {}:", session_name);
        
        match cache.summarize_session(session_id) {
            Ok(summary) => {
                println!("   📝 Summary: {}", summary.summary_text);
                println!("   🏷️  Key Topics: {}", summary.key_topics.join(", "));
                println!("   📊 {} memories spanning {} to {}", 
                        summary.memory_count,
                        summary.date_range.0.format("%H:%M"),
                        summary.date_range.1.format("%H:%M"));
                println!("   ⭐ Average Importance: {:.1}", summary.importance_score);
            },
            Err(e) => println!("   ❌ Error generating summary: {}", e),
        }
        println!();
    }

    // Cross-session search examples
    println!("🔍 Cross-Session Search Examples...\n");

    // Search for risk-related content across all sessions
    println!("1. Risk-related memories across all sessions:");
    let risk_memories = cache.recall(user_id, Some("risk"), None, Some(5))?;
    for memory in &risk_memories {
    let session_name = sessions.iter()
        .find(|s| s.id == memory.session_id)
        .and_then(|s| s.name.as_ref())
        .map(|s| s.as_str())  // Convert &String to &str
        .unwrap_or("Unknown");
        println!("   📁 [{}] {}", session_name, memory.content);
    }
    println!();

    // Search for high-importance memories
    println!("2. High-importance memories (>0.7):");
    let filter = QueryFilter {
        user_id: Some(user_id.to_string()),
        session_id: None,
        keywords: None,
        date_from: None,
        date_to: None,
        limit: Some(5),
        min_importance: Some(0.7),
    };

    let important_memories = cache.recall_advanced(filter)?;
    for memory in &important_memories {
        let session_name = sessions.iter()
            .find(|s| s.id == memory.session_id)
            .and_then(|s| s.name.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Unknown");
        println!("   ⭐ [{:.1}] [{}] {}", memory.importance, session_name, memory.content);
    }
    println!();

    // Session-based search
    println!("3. Find sessions containing 'AAPL':");
    let apple_sessions = cache.search_sessions(user_id, vec!["AAPL".to_string()])?;
    for session in &apple_sessions {
        println!("   📁 {} ({} memories)", 
                session.name.as_ref().unwrap_or(&"Unnamed".to_string()),
                session.memory_count);
    }
    println!();

    // Memory timeline analysis
    println!("⏰ Memory Timeline Analysis...\n");
    
    let all_memories = cache.recall(user_id, None, None, None)?;
    let mut session_timeline: HashMap<String, Vec<chrono::DateTime<Utc>>> = HashMap::new();
    
    for memory in &all_memories {
        session_timeline
            .entry(memory.session_id.clone())
            .or_insert_with(Vec::new)
            .push(memory.timestamp);
    }

    for (session_id, timestamps) in session_timeline {
        let session_name = sessions.iter()
            .find(|s| s.id == session_id)
            .and_then(|s| s.name.as_ref())
            .map(|s| s.as_str())
            .unwrap_or("Unknown");
            
        let min_time = timestamps.iter().min().unwrap();
        let max_time = timestamps.iter().max().unwrap();
        let duration = *max_time - *min_time;
        
        println!("   📁 {}: {} memories over {} minutes", 
                session_name, 
                timestamps.len(),
                duration.num_minutes());
    }
    println!();

    // Export session-specific data
    println!("📤 Session Export Example...\n");
    
    // Get memories for specific session
    let trading_memories = cache.get_session_memories(user_id, &trading_session)?;
    println!("Trading session export ({} memories):", trading_memories.len());
    for (i, memory) in trading_memories.iter().take(2).enumerate() {
        println!("   {}. {}", i + 1, memory.content);
        println!("      Metadata: {:?}", memory.metadata);
    }
    if trading_memories.len() > 2 {
        println!("   ... and {} more", trading_memories.len() - 2);
    }

    println!("\n✅ Session management example completed!");
    println!("💡 Data stored in: ./session_example_data");
    println!("🧹 Cleanup: rm -rf ./session_example_data");

    Ok(())
}