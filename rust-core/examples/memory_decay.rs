//! Memory decay and compression example
//! 
//! Demonstrates:
//! - Automatic memory expiration
//! - Memory compression and summarization
//! - Custom decay policies
//! - Storage optimization

use mindcache_core::{MindCache, MindCacheConfig, DecayPolicy};
use std::collections::HashMap;
use chrono::{Utc, Duration};
use std::thread::sleep;
use std::time::Duration as StdDuration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    println!("🧹 MindCache Memory Decay Example");
    println!("=================================\n");

    // Create cache with aggressive decay for demonstration
    let config = MindCacheConfig {
        storage_path: "./decay_example_data".to_string(),
        auto_decay_enabled: true,
        decay_interval_hours: 1,
        default_memory_ttl_hours: Some(1), // Very short for demo
        enable_compression: true,
        max_memories_per_user: 50, // Low limit for demo
        importance_threshold: 0.4,
    };

    let mut cache = MindCache::with_config(config.clone())?;
    println!("✅ MindCache initialized with aggressive decay policy\n");

    let user_id = "decay_demo_user";
    let session_id = cache.create_session(user_id, Some("Decay Demo Session"))?;

    // Create memories with different importance levels and TTL
    println!("💾 Creating memories with varying importance and TTL...\n");

    let memories_data = vec![
        // High importance, long TTL - should survive
        ("Critical trade alert: AAPL earnings beat expectations. Stock up 8% AH.", 0.9, Some(24)),
        ("BREAKING: Fed announces emergency rate cut. Markets rallying.", 0.95, Some(48)),
        ("Personal reminder: Tax deadline April 15th. Schedule appointment with CPA.", 0.8, Some(168)),
        
        // Medium importance, default TTL
        ("Tesla production numbers look strong this quarter.", 0.6, None),
        ("Oil prices trending higher due to supply concerns.", 0.5, None),
        ("Crypto market showing signs of institutional adoption.", 0.7, None),
        
        // Low importance, short TTL - should expire quickly
        ("Coffee shop was crowded today. Market sentiment seems positive.", 0.2, Some(1)),
        ("Checked portfolio. Everything looking stable.", 0.1, Some(1)),
        ("Random thought: should diversify more into international markets.", 0.3, Some(2)),
        ("Weather is nice today. Good for market psychology.", 0.1, Some(1)),
        
        // Medium-low importance, various TTL - candidates for compression
        ("Apple stock hit new 52-week high today.", 0.4, None),
        ("Microsoft earnings call was boring but solid.", 0.3, None),
        ("Google advertising revenue growth slowing.", 0.4, None),
        ("Amazon logistics costs are concerning investors.", 0.35, None),
        ("Netflix subscriber growth disappointing.", 0.3, None),
    ];

    for (content, importance, ttl) in &memories_data {
        let mut metadata = HashMap::new();
        metadata.insert("importance_category".to_string(), 
                       if *importance > 0.7 { "high" } 
                       else if *importance > 0.4 { "medium" } 
                       else { "low" }.to_string());
        
        cache.save_with_options(user_id, &session_id, content, Some(metadata), *importance, *ttl)?;
        
        // Small delay to create different timestamps
        sleep(StdDuration::from_millis(100));
    }

    println!("✅ Created {} memories with varying importance levels\n", memories_data.len());

    // Show initial statistics
    let initial_stats = cache.get_stats();
    println!("📊 Initial Statistics:");
    if let Some(storage_stats) = initial_stats.get("storage") {
        println!("   💾 {}", serde_json::to_string_pretty(storage_stats)?);
    }
    println!();

    // Wait a bit to let some memories "age"
    println!("⏰ Waiting 2 seconds for memories to age...\n");
    sleep(StdDuration::from_secs(2));

    // Show memories before decay
    println!("📋 All memories before decay:");
    let all_memories = cache.recall(user_id, None, None, None)?;
    for (i, memory) in all_memories.iter().enumerate() {
        let age_seconds = (Utc::now() - memory.timestamp).num_seconds();
        println!("   {}. [⭐{:.1}] [{}s old] {}", 
                i + 1, 
                memory.importance, 
                age_seconds,
                if memory.content.len() > 50 { 
                    format!("{}...", &memory.content[..50]) 
                } else { 
                    memory.content.clone() 
                });
    }
    println!("   Total: {} memories\n", all_memories.len());

    // Demonstrate custom decay policy
    println!("⚙️ Setting up custom decay policy...\n");
    
    let custom_policy = DecayPolicy {
        max_age_hours: 0, // Very aggressive - expire everything older than 0 hours
        importance_threshold: 0.5, // Only preserve high importance
        max_memories_per_user: 20,
        compression_enabled: true,
        auto_summarize_sessions: true,
    };

    // Update cache with custom decay policy
    let updated_config = MindCacheConfig {
        importance_threshold: custom_policy.importance_threshold,
        max_memories_per_user: custom_policy.max_memories_per_user,
        enable_compression: custom_policy.compression_enabled,
        ..config
    };
    cache.update_config(updated_config)?;

    // Run decay process
    println!("🧹 Running memory decay process...\n");
    let decay_stats = cache.decay()?;

    println!("📊 Decay Results:");
    println!("   🗑️  Memories expired: {}", decay_stats.memories_expired);
    println!("   📦 Memories compressed: {}", decay_stats.memories_compressed);
    println!("   📋 Sessions summarized: {}", decay_stats.sessions_summarized);
    println!("   📈 Before decay: {} memories", decay_stats.total_memories_before);
    println!("   📉 After decay: {} memories", decay_stats.total_memories_after);
    println!("   💾 Storage saved: {} bytes", decay_stats.storage_saved_bytes);
    println!("   ⏰ Process completed at: {}", decay_stats.last_decay_run.format("%H:%M:%S"));
    println!();

    // Show surviving memories
    println!("🏆 Surviving memories after decay:");
    let surviving_memories = cache.recall(user_id, None, None, None)?;
    if surviving_memories.is_empty() {
        println!("   📭 No memories survived the decay process");
    } else {
        for (i, memory) in surviving_memories.iter().enumerate() {
            println!("   {}. [⭐{:.1}] {}", i + 1, memory.importance, memory.content);
            if let Some(ttl) = memory.ttl_hours {
                println!("      TTL: {} hours", ttl);
            }
        }
    }
    println!("   Total surviving: {} memories\n", surviving_memories.len());

    // Demonstrate memory age analysis
    println!("📈 Memory Age Distribution Analysis...\n");
    
    // Add some new memories with different ages (simulated)
    let test_memories = vec![
        "New memory just added",
        "Another fresh memory",
        "One more recent memory",
    ];

    for content in &test_memories {
        cache.save(user_id, &session_id, content, None)?;
    }

    // Note: In a real implementation, you'd have memories with actual age differences
    println!("   📊 Age distribution analysis would show:");
    println!("   • 0-24h: {} memories", test_memories.len());
    println!("   • 1-7d: 0 memories");
    println!("   • 1-4w: 0 memories");
    println!("   • 1-3m: 0 memories");
    println!("   • 3m+: 0 memories");
    println!();

    // Show session summary after decay
    println!("📋 Session Summary After Decay...\n");
    match cache.summarize_session(&session_id) {
        Ok(summary) => {
            println!("   📝 Summary: {}", summary.summary_text);
            println!("   🏷️  Key Topics: {}", summary.key_topics.join(", "));
            println!("   📊 Memory Count: {}", summary.memory_count);
            println!("   ⭐ Importance Score: {:.2}", summary.importance_score);
        },
        Err(e) => println!("   ❌ Could not generate summary: {}", e),
    }
    println!();

    // Demonstrate different decay scenarios
    println!("🎯 Testing Different Decay Scenarios...\n");

    // Scenario 1: Memory limit enforcement
    println!("Scenario 1: Memory limit enforcement");
    for i in 0..30 {
        cache.save(user_id, &session_id, 
                  &format!("Bulk memory #{} for limit testing", i), None)?;
    }
    
    let before_limit = cache.recall(user_id, None, None, None)?.len();
    let limit_decay = cache.decay()?;
    let after_limit = cache.recall(user_id, None, None, None)?.len();
    
    println!("   Before: {} memories", before_limit);
    println!("   After: {} memories", after_limit);
    println!("   Removed: {} memories due to limits", limit_decay.memories_expired);
    println!();

// Add memories with varying importance
   let importance_test_memories = vec![
       ("Ultra important market crash warning!", 1.0),
       ("Very important Fed decision tomorrow", 0.9),
       ("Important earnings report next week", 0.7),
       ("Somewhat interesting market observation", 0.4),
       ("Minor portfolio adjustment note", 0.2),
       ("Trivial daily market comment", 0.1),
   ];

   for (content, importance) in &importance_test_memories {
       cache.save_with_options(user_id, &session_id, content, None, *importance, Some(1))?;
   }

   let importance_decay = cache.decay()?;
   let final_memories = cache.recall(user_id, None, None, None)?;
   
   println!("   Added {} memories with varying importance", importance_test_memories.len());
   println!("   Expired {} low-importance memories", importance_decay.memories_expired);
   println!("   Final memory count: {}", final_memories.len());
   
   if !final_memories.is_empty() {
       println!("   Surviving memories (by importance):");
       let mut sorted_memories = final_memories;
       sorted_memories.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap());
       for memory in sorted_memories.iter().take(5) {
           println!("     • [⭐{:.1}] {}", memory.importance, 
                   if memory.content.len() > 40 { 
                       format!("{}...", &memory.content[..40]) 
                   } else { 
                       memory.content.clone() 
                   });
       }
   }
   println!();

   // Final statistics
   println!("📊 Final System Statistics...\n");
   let final_stats = cache.get_stats();
   
   if let Some(storage_stats) = final_stats.get("storage") {
       println!("Storage Stats:");
       println!("   {}", serde_json::to_string_pretty(storage_stats)?);
   }
   
   if let Some(session_stats) = final_stats.get("sessions") {
       println!("Session Stats:");
       println!("   {}", serde_json::to_string_pretty(session_stats)?);
   }
   
   if let Some(decay_stats) = final_stats.get("decay") {
       println!("Decay Stats:");
       println!("   {}", serde_json::to_string_pretty(decay_stats)?);
   }

   println!("\n✅ Memory decay example completed!");
   println!("🔍 Key Insights:");
   println!("   • High-importance memories survive longer");
   println!("   • TTL settings override default decay rules");
   println!("   • Memory limits enforce storage boundaries");
   println!("   • Compression helps preserve information while saving space");
   println!("   • Session summaries provide context after individual memories decay");
   
   println!("\n💡 Data stored in: ./decay_example_data");
   println!("🧹 Cleanup: rm -rf ./decay_example_data");

   Ok(())
}