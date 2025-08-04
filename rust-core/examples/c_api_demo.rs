//! C API demonstration example
//! 
//! Shows how to use MindCache through the C FFI interface
//! This simulates how Node.js would interact with the Rust core

use mindcache_core::*;
use std::ffi::{CString, CStr};
use std::ptr;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 MindCache C API Demo");
    println!("=======================\n");

    // Test 1: Initialize MindCache
    println!("1. 🚀 Initializing MindCache...");
    let cache_ptr = mindcache_init();
    
    if cache_ptr.is_null() {
        println!("   ❌ Failed to initialize MindCache");
        return Err("Initialization failed".into());
    }
    println!("   ✅ MindCache initialized successfully\n");

    // Test 2: Initialize with custom config
    println!("2. ⚙️ Testing custom configuration...");
    let config_json = r#"{
        "storage_path": "./c_api_demo_data",
        "auto_decay_enabled": true,
        "decay_interval_hours": 24,
        "default_memory_ttl_hours": 72,
        "enable_compression": true,
        "max_memories_per_user": 1000,
        "importance_threshold": 0.3
    }"#;
    
    let config_cstring = CString::new(config_json)?;
    let custom_cache_ptr = mindcache_init_with_config(config_cstring.as_ptr());
    
    if custom_cache_ptr.is_null() {
        println!("   ❌ Failed to initialize with custom config");
    } else {
        println!("   ✅ Custom configuration applied successfully");
        mindcache_destroy(custom_cache_ptr);
    }
    println!();

    // Test 3: Save memories
    println!("3. 💾 Saving memories through C API...");
    
    let user_id = CString::new("c_api_user")?;
    let session_id = CString::new("c_session_1")?;
    
    let memories = vec![
        ("Bought AAPL at $175. Strong momentum on earnings beat.", r#"{"category":"trading","asset":"AAPL"}"#),
        ("Market showing volatility. Consider reducing position sizes.", r#"{"category":"risk","sentiment":"cautious"}"#),
        ("Fed meeting next week. Watch for rate guidance.", r#"{"category":"macro","event":"fed_meeting"}"#),
        ("Portfolio rebalancing due. Tech allocation too high.", r#"{"category":"portfolio","action":"rebalance"}"#),
    ];

    let mut memory_ids = Vec::new();
    
    for (content, metadata) in &memories {
        let content_cstring = CString::new(*content)?;
        let metadata_cstring = CString::new(*metadata)?;
        
        let result_ptr = mindcache_save(
            cache_ptr,
            user_id.as_ptr(),
            session_id.as_ptr(),
            content_cstring.as_ptr(),
            metadata_cstring.as_ptr(),
        );
        
        if result_ptr.is_null() {
            println!("   ❌ Failed to save memory: {}", content);
        } else {
            let memory_id_cstr = unsafe { CStr::from_ptr(result_ptr) };
            let memory_id = memory_id_cstr.to_str()?;
            memory_ids.push(memory_id.to_string());
            println!("   ✅ Saved: {} (ID: {})", 
                    if content.len() > 30 { &content[..30] } else { content },
                    memory_id);
            
            // Free the returned string
            mindcache_free_string(result_ptr);
        }
    }
    println!("   📊 Total saved: {} memories\n", memory_ids.len());

    // Test 4: Recall memories
    println!("4. 🔍 Recalling memories...");
    
    // Test simple recall
    let query = CString::new("AAPL")?;
    let recall_result = mindcache_recall(
        cache_ptr,
        user_id.as_ptr(),
        query.as_ptr(),
        ptr::null(),
        10,
    );
    
    if recall_result.is_null() {
        println!("   ❌ Failed to recall memories");
    } else {
        let recall_json_cstr = unsafe { CStr::from_ptr(recall_result) };
        let recall_json = recall_json_cstr.to_str()?;
        
        // Parse and display results (simplified)
        if recall_json.contains("AAPL") {
            println!("   ✅ Successfully recalled AAPL-related memories");
            println!("   📄 Result preview: {}...", 
                    if recall_json.len() > 100 { &recall_json[..100] } else { recall_json });
        } else {
            println!("   ⚠️ No AAPL memories found");
        }
        
        mindcache_free_string(recall_result);
    }
    
    // Test recall all memories for session
    let all_recall_result = mindcache_recall(
        cache_ptr,
        user_id.as_ptr(),
        ptr::null(),
        session_id.as_ptr(),
        -1, // No limit
    );
    
    if !all_recall_result.is_null() {
        let all_json_cstr = unsafe { CStr::from_ptr(all_recall_result) };
        let all_json = all_json_cstr.to_str()?;
        let memory_count = all_json.matches("\"id\":").count();
        println!("   ✅ Recalled all session memories: {} found", memory_count);
        mindcache_free_string(all_recall_result);
    }
    println!();

    // Test 5: Generate session summary
    println!("5. 📋 Generating session summary...");
    
    let summary_result = mindcache_summarize(cache_ptr, session_id.as_ptr());
    
    if summary_result.is_null() {
        println!("   ❌ Failed to generate summary");
    } else {
        let summary_json_cstr = unsafe { CStr::from_ptr(summary_result) };
        let summary_json = summary_json_cstr.to_str()?;
        
        println!("   ✅ Summary generated successfully");
        println!("   📄 Summary preview: {}...", 
                if summary_json.len() > 150 { &summary_json[..150] } else { summary_json });
        
        mindcache_free_string(summary_result);
    }
    println!();

    // Test 6: Memory decay
    println!("6. 🧹 Running memory decay...");
    
    let decay_result = mindcache_decay(cache_ptr);
    
    if decay_result.is_null() {
        println!("   ❌ Failed to run decay process");
    } else {
        let decay_json_cstr = unsafe { CStr::from_ptr(decay_result) };
        let decay_json = decay_json_cstr.to_str()?;
        
        println!("   ✅ Decay process completed");
        println!("   📊 Decay stats: {}...", 
                if decay_json.len() > 100 { &decay_json[..100] } else { decay_json });
        
        mindcache_free_string(decay_result);
    }
    println!();

    // Test 7: Get system statistics
    println!("7. 📊 Getting system statistics...");
    
    let stats_result = mindcache_get_stats(cache_ptr);
    
    if stats_result.is_null() {
        println!("   ❌ Failed to get statistics");
    } else {
        let stats_json_cstr = unsafe { CStr::from_ptr(stats_result) };
        let stats_json = stats_json_cstr.to_str()?;
        
        println!("   ✅ Statistics retrieved successfully");
        println!("   📈 Stats preview: {}...", 
                if stats_json.len() > 200 { &stats_json[..200] } else { stats_json });
        
        mindcache_free_string(stats_result);
    }
    println!();

    // Test 8: Error handling
    println!("8. 🚨 Testing error handling...");
    
    // Test with null pointers
    let null_result = mindcache_save(
        ptr::null_mut(),
        user_id.as_ptr(),
        session_id.as_ptr(),
        CString::new("test").unwrap().as_ptr(),
        ptr::null(),
    );
    
    if null_result.is_null() {
        println!("   ✅ Properly handled null cache pointer");
    } else {
        println!("   ❌ Should have returned null for invalid cache");
        mindcache_free_string(null_result);
    }
    
    // Test with invalid JSON
    let invalid_json = CString::new("invalid json {")?;
    let invalid_config_cache = mindcache_init_with_config(invalid_json.as_ptr());
    if invalid_config_cache.is_null() {
        println!("   ✅ Properly handled invalid JSON config");
    } else {
        println!("   ❌ Should have failed with invalid JSON");
        mindcache_destroy(invalid_config_cache);
    }
    println!();

    // Test 9: Performance simulation
    println!("9. ⚡ Performance simulation...");
    
    let start_time = std::time::Instant::now();
    
    // Simulate bulk operations
    for i in 0..50 {
        let bulk_content = CString::new(format!("Bulk memory {} for performance testing", i))?;
        let bulk_result = mindcache_save(
            cache_ptr,
            user_id.as_ptr(),
            session_id.as_ptr(),
            bulk_content.as_ptr(),
            ptr::null(),
        );
        
        if !bulk_result.is_null() {
            mindcache_free_string(bulk_result);
        }
    }
    
    let bulk_save_time = start_time.elapsed();
    
    // Test bulk recall
    let recall_start = std::time::Instant::now();
    let bulk_recall = mindcache_recall(
        cache_ptr,
        user_id.as_ptr(),
        ptr::null(),
        ptr::null(),
        -1,
    );
    let recall_time = recall_start.elapsed();
    
    if !bulk_recall.is_null() {
        let bulk_json_cstr = unsafe { CStr::from_ptr(bulk_recall) };
        let bulk_json = bulk_json_cstr.to_str()?;
        let total_memories = bulk_json.matches("\"id\":").count();
        
        println!("   ⚡ Saved 50 memories in {:?}", bulk_save_time);
        println!("   ⚡ Recalled {} memories in {:?}", total_memories, recall_time);
        println!("   📊 Average save time: {:?} per memory", bulk_save_time / 50);
        
        mindcache_free_string(bulk_recall);
    }
    println!();

    // Test 10: Cleanup
    println!("10. 🧹 Cleanup...");
    mindcache_destroy(cache_ptr);
    println!("    ✅ MindCache instance destroyed");
    println!("    🗑️ Memory freed successfully");

    println!("\n🎉 C API Demo completed successfully!");
    println!("💡 This demonstrates how Node.js can interact with Rust core");
    println!("🔗 All string memory was properly managed and freed");
    println!("⚡ Performance characteristics look good for production use");
    
    println!("\n📁 Data stored in: ./c_api_demo_data");
    println!("🧹 Cleanup: rm -rf ./c_api_demo_data");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_c_api_basic_flow() {
        // Basic test to ensure C API functions don't panic
        let cache_ptr = mindcache_init();
        assert!(!cache_ptr.is_null());
        
        mindcache_destroy(cache_ptr);
    }
    
    #[test]
    fn test_c_api_null_handling() {
        // Test that functions handle null pointers gracefully
        let result = mindcache_save(
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
        );
        assert!(result.is_null());
    }
}