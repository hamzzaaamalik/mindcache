//! Basic usage example for MindCache Rust API

use mindcache_core::{MindCache, MindCacheConfig}; 

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    #[cfg(feature = "logging")]
    env_logger::init();
    
    println!("🧠 MindCache Basic Usage Example");
    println!("================================\n");

    // Create MindCache with custom config
    let config = MindCacheConfig {
        storage_path: "./example_data".to_string(),
        auto_decay_enabled: true,
        decay_interval_hours: 1,
        default_memory_ttl_hours: Some(48),
        enable_compression: true,
        max_memories_per_user: 1000,
        importance_threshold: 0.2,
    };

    let mut cache = MindCache::with_config(config)?;
    println!("✅ MindCache initialized with custom config\n");

    // Create a session
    let session_id = cache.create_session("test_user", Some("Test Session"))?;
    
    // Save memories
    let memory_id = cache.save("test_user", &session_id, "Test memory content", None)?;
    println!("✅ Saved memory: {}", memory_id);
    
    // Recall memory
    let memories = cache.recall("test_user", Some("Test"), None, None)?;
    println!("✅ Recalled {} memories", memories.len());
    
    for memory in &memories {
        println!("   📝 {}", memory.content);
    }

    println!("\n✅ Example completed successfully!");
    println!("🧹 Cleanup: rm -rf ./example_data");
    Ok(())
}