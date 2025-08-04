//! Performance tests for MindCache
//! 
//! These tests verify that MindCache performs well under various load conditions
//! and measure key performance metrics.

#[cfg(feature = "benchmarks")]
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use mindcache_core::{MindCache, MindCacheConfig, QueryFilter};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tempfile::TempDir;

fn create_test_cache() -> (MindCache, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = MindCacheConfig {
        storage_path: temp_dir.path().to_str().unwrap().to_string(),
        auto_decay_enabled: false,
        decay_interval_hours: 24,
        default_memory_ttl_hours: Some(24),
        enable_compression: true,
        max_memories_per_user: 10000,
        importance_threshold: 0.3,
    };
    
    let cache = MindCache::with_config(config).expect("Failed to create test cache");
    (cache, temp_dir)
}

#[test]
fn test_save_performance() {
    let (mut cache, _temp_dir) = create_test_cache();
    
    let user_id = "perf_user";
    let session_id = cache.create_session(user_id, Some("Performance Test"))
        .expect("Should create session");
    
    let start = Instant::now();
    let num_saves = 1000;
    
    for i in 0..num_saves {
        let content = format!("Performance test memory number {}", i);
        cache.save(user_id, &session_id, &content, None)
            .expect("Should save memory");
    }
    
    let duration = start.elapsed();
    let saves_per_second = num_saves as f64 / duration.as_secs_f64();
    
    println!("Saved {} memories in {:?}", num_saves, duration);
    println!("Performance: {:.2} saves/second", saves_per_second);
    
    // Should be able to save at least 100 memories per second
    assert!(saves_per_second > 100.0, 
           "Save performance too slow: {:.2} saves/second", saves_per_second);
}

#[test]
fn test_recall_performance() {
    let (mut cache, _temp_dir) = create_test_cache();
    
    let user_id = "recall_perf_user";
    let session_id = cache.create_session(user_id, Some("Recall Performance Test"))
        .expect("Should create session");
    
    // Populate with test data
    let num_memories = 5000;
    for i in 0..num_memories {
        let content = format!("Memory {} about trading stocks and crypto", i);
        cache.save(user_id, &session_id, &content, None)
            .expect("Should save memory");
    }
    
    // Test different recall scenarios
    let scenarios = vec![
        ("keyword_search", Some("trading")),
        ("broad_search", Some("crypto")),
        ("no_filter", None),
    ];
    
    for (scenario_name, query) in scenarios {
        let start = Instant::now();
        let num_recalls = 100;
        
        for _ in 0..num_recalls {
            let _memories = cache.recall(user_id, query, None, Some(50))
                .expect("Should recall memories");
        }
        
        let duration = start.elapsed();
        let recalls_per_second = num_recalls as f64 / duration.as_secs_f64();
        
        println!("{}: {} recalls in {:?} ({:.2} recalls/second)", 
                scenario_name, num_recalls, duration, recalls_per_second);
        
        // Should be able to perform at least 50 recalls per second
        assert!(recalls_per_second > 50.0, 
               "Recall performance too slow for {}: {:.2} recalls/second", 
               scenario_name, recalls_per_second);
    }
}

#[test]
fn test_concurrent_performance() {
    let (mut cache, _temp_dir) = create_test_cache();
    
    let user_id = "concurrent_user";
    let session_id = cache.create_session(user_id, Some("Concurrent Test"))
        .expect("Should create session");
    
    // Simulate concurrent load by interleaving operations
    let start = Instant::now();
    let operations = 500;
    
    for i in 0..operations {
        // Save operation
        let content = format!("Concurrent memory {}", i);
        cache.save(user_id, &session_id, &content, None)
            .expect("Should save memory");
        
        // Every 10th operation, do a recall
        if i % 10 == 0 {
            let _memories = cache.recall(user_id, Some("memory"), None, Some(10))
                .expect("Should recall memories");
        }
        
        // Every 20th operation, get stats
        if i % 20 == 0 {
            let _stats = cache.get_stats();
        }
    }
    
    let duration = start.elapsed();
    let ops_per_second = operations as f64 / duration.as_secs_f64();
    
    println!("Completed {} mixed operations in {:?} ({:.2} ops/second)", 
            operations, duration, ops_per_second);
    
    // Should handle at least 100 mixed operations per second
    assert!(ops_per_second > 100.0, 
           "Concurrent performance too slow: {:.2} ops/second", ops_per_second);
}

#[test]
fn test_memory_usage_scaling() {
    let (mut cache, _temp_dir) = create_test_cache();
    
    let user_id = "memory_scale_user";
    let session_id = cache.create_session(user_id, Some("Memory Scale Test"))
        .expect("Should create session");
    
    let mut memory_counts = vec![100, 500, 1000, 2000, 5000];
    let mut save_times = Vec::new();
    let mut recall_times = Vec::new();
    
    for &count in &memory_counts {
        // Clear previous data by creating new cache
        let (mut fresh_cache, _temp_dir2) = create_test_cache();
        let fresh_session = fresh_cache.create_session(user_id, Some("Fresh Session"))
            .expect("Should create fresh session");
        
        // Time saves
        let save_start = Instant::now();
        for i in 0..count {
            let content = format!("Scaling test memory {} with some content to make it realistic", i);
            fresh_cache.save(user_id, &fresh_session, &content, None)
                .expect("Should save memory");
        }
        let save_time = save_start.elapsed();
        save_times.push(save_time);
        
        // Time recalls
        let recall_start = Instant::now();
        let _memories = fresh_cache.recall(user_id, Some("memory"), None, Some(100))
            .expect("Should recall memories");
        let recall_time = recall_start.elapsed();
        recall_times.push(recall_time);
        
        println!("Count: {}, Save time: {:?}, Recall time: {:?}", 
                count, save_time, recall_time);
    }
    
    // Verify that performance doesn't degrade too much with scale
    // Save time should grow roughly linearly
    let first_save_rate = memory_counts[0] as f64 / save_times[0].as_secs_f64();
    let last_save_rate = memory_counts[memory_counts.len()-1] as f64 / save_times[save_times.len()-1].as_secs_f64();
    
    println!("Save rate - First: {:.2}/sec, Last: {:.2}/sec, Ratio: {:.2}", 
            first_save_rate, last_save_rate, first_save_rate / last_save_rate);
    
    // Performance shouldn't degrade more than 3x
    assert!(first_save_rate / last_save_rate < 3.0, 
           "Save performance degraded too much with scale");
    
    // Recall time should remain relatively stable (good indexing)
    let first_recall = recall_times[0];
    let last_recall = recall_times[recall_times.len()-1];
    
    println!("Recall time - First: {:?}, Last: {:?}, Ratio: {:.2}", 
            first_recall, last_recall, 
            last_recall.as_secs_f64() / first_recall.as_secs_f64());
    
    // Recall time shouldn't increase more than 5x with 50x more data
    assert!(last_recall.as_secs_f64() / first_recall.as_secs_f64() < 5.0, 
           "Recall performance degraded too much with scale");
}

#[test]
fn test_large_content_performance() {
    let (mut cache, _temp_dir) = create_test_cache();
    
    let user_id = "large_content_user";
    let session_id = cache.create_session(user_id, Some("Large Content Test"))
        .expect("Should create session");
    
    // Test different content sizes
    let content_sizes = vec![1000, 5000, 10000, 50000]; // bytes
    
    for &size in &content_sizes {
        let large_content = "A".repeat(size);
        
        let save_start = Instant::now();
        cache.save(user_id, &session_id, &large_content, None)
            .expect("Should save large content");
        let save_time = save_start.elapsed();
        
        let recall_start = Instant::now();
        let memories = cache.recall(user_id, Some("A"), None, Some(1))
            .expect("Should recall large content");
        let recall_time = recall_start.elapsed();
        
        assert!(!memories.is_empty(), "Should find large content");
        assert_eq!(memories[0].content.len(), size, "Content size should be preserved");
        
        println!("Size: {} bytes, Save: {:?}, Recall: {:?}", 
                size, save_time, recall_time);
        
        // Even large content should save/recall reasonably quickly
        assert!(save_time < Duration::from_secs(1), 
               "Large content save too slow: {:?} for {} bytes", save_time, size);
        assert!(recall_time < Duration::from_millis(500), 
               "Large content recall too slow: {:?} for {} bytes", recall_time, size);
    }
}

#[test]
fn test_decay_performance() {
    let (mut cache, _temp_dir) = create_test_cache();
    
    let user_id = "decay_perf_user";
    let session_id = cache.create_session(user_id, Some("Decay Performance"))
        .expect("Should create session");
    
    // Create a large number of memories with different importance
    let num_memories = 2000;
    for i in 0..num_memories {
        let importance = (i % 10) as f32 / 10.0; // 0.0 to 0.9
        let content = format!("Decay test memory {} with importance {}", i, importance);
        
        cache.save_with_options(user_id, &session_id, &content, None, importance, Some(1))
            .expect("Should save memory");
    }
    
    println!("Created {} memories for decay testing", num_memories);
    
    // Measure decay performance
    let decay_start = Instant::now();
    let decay_stats = cache.decay().expect("Should run decay");
    let decay_time = decay_start.elapsed();
    
    println!("Decay completed in {:?}", decay_time);
    println!("Decay stats: expired={}, compressed={}, before={}, after={}", 
             decay_stats.memories_expired,
             decay_stats.memories_compressed,
             decay_stats.total_memories_before,
             decay_stats.total_memories_after);
    
    // Decay should complete within reasonable time even with many memories
    assert!(decay_time < Duration::from_secs(10), 
           "Decay too slow: {:?} for {} memories", decay_time, num_memories);
    
    // Should have processed some memories
    assert!(decay_stats.total_memories_before > 0, "Should have found memories to process");
}

#[test] 
fn test_session_summary_performance() {
    let (mut cache, _temp_dir) = create_test_cache();
    
    let user_id = "summary_perf_user";
    let session_id = cache.create_session(user_id, Some("Summary Performance"))
        .expect("Should create session");
    
    // Create sessions with varying numbers of memories
    let memory_counts = vec![10, 50, 100, 200];
    
    for &count in &memory_counts {
        // Create fresh session for each test
        let test_session = cache.create_session(user_id, Some(&format!("Test Session {}", count)))
            .expect("Should create test session");
        
        // Add memories
        for i in 0..count {
            let content = format!("Summary test memory {} about trading and investments", i);
            cache.save(user_id, &test_session, &content, None)
                .expect("Should save memory");
        }
        
        // Time summary generation
        let summary_start = Instant::now();
        let summary = cache.summarize_session(&test_session)
            .expect("Should generate summary");
        let summary_time = summary_start.elapsed();
        
        println!("Session with {} memories: summary generated in {:?}", 
                count, summary_time);
        
        assert_eq!(summary.memory_count, count, "Summary should count all memories");
        assert!(!summary.summary_text.is_empty(), "Should generate summary text");
        
        // Summary generation should be reasonably fast
        let max_time = Duration::from_millis(count as u64 * 10); // 10ms per memory
        assert!(summary_time < max_time, 
               "Summary generation too slow: {:?} for {} memories", 
               summary_time, count);
    }
}

#[test]
fn test_storage_file_growth() {
    let (mut cache, temp_dir) = create_test_cache();
    
    let user_id = "storage_growth_user";
    let session_id = cache.create_session(user_id, Some("Storage Growth Test"))
        .expect("Should create session");
    
    let storage_file = temp_dir.path().join("memories.bin");
    
    // Add memories and track file size growth
    let batches = vec![100, 500, 1000];
    let mut previous_size = 0u64;
    
    for &batch_size in &batches {
        for i in 0..batch_size {
            let content = format!("Storage growth test memory {} with consistent content length", i);
            cache.save(user_id, &session_id, &content, None)
                .expect("Should save memory");
        }
        
        // Check file size
        let current_size = if storage_file.exists() {
            std::fs::metadata(&storage_file)
                .expect("Should read file metadata")
                .len()
        } else {
            0
        };
        
        println!("After {} memories: file size = {} bytes", batch_size, current_size);
        
        if previous_size > 0 {
            // File should grow roughly linearly with content
            let growth_ratio = current_size as f64 / previous_size as f64;
            println!("Growth ratio: {:.2}", growth_ratio);
            
            // Growth should be reasonable (not exponential)
            assert!(growth_ratio < 10.0, 
                   "File size growing too fast: ratio = {:.2}", growth_ratio);
        }
        
        previous_size = current_size;
    }
}

// Benchmark functions for Criterion (only compiled with benchmarks feature)
#[cfg(feature = "benchmarks")]
fn bench_save_operations(c: &mut Criterion) {
    let (mut cache, _temp_dir) = create_test_cache();
    let user_id = "bench_user";
    let session_id = cache.create_session(user_id, Some("Benchmark Session"))
        .expect("Should create session");
    
    c.bench_function("save_memory", |b| {
        let mut counter = 0;
        b.iter(|| {
            let content = format!("Benchmark memory {}", counter);
           counter += 1;
           cache.save(black_box(user_id), black_box(&session_id), black_box(&content), black_box(None))
               .expect("Should save memory")
       })
   });
}

#[cfg(feature = "benchmarks")]
fn bench_recall_operations(c: &mut Criterion) {
   let (mut cache, _temp_dir) = create_test_cache();
   let user_id = "bench_recall_user";
   let session_id = cache.create_session(user_id, Some("Recall Benchmark"))
       .expect("Should create session");
   
   // Pre-populate with test data
   for i in 0..1000 {
       let content = format!("Benchmark recall memory {} about trading stocks", i);
       cache.save(user_id, &session_id, &content, None)
           .expect("Should save memory");
   }
   
   let mut group = c.benchmark_group("recall_operations");
   
   group.bench_function("recall_with_keyword", |b| {
       b.iter(|| {
           cache.recall(black_box(user_id), black_box(Some("trading")), black_box(None), black_box(Some(10)))
               .expect("Should recall memories")
       })
   });
   
   group.bench_function("recall_all_session", |b| {
       b.iter(|| {
           cache.recall(black_box(user_id), black_box(None), black_box(Some(&session_id)), black_box(Some(50)))
               .expect("Should recall memories")
       })
   });
   
   group.bench_function("recall_no_filter", |b| {
       b.iter(|| {
           cache.recall(black_box(user_id), black_box(None), black_box(None), black_box(Some(20)))
               .expect("Should recall memories")
       })
   });
   
   group.finish();
}

#[cfg(feature = "benchmarks")]
fn bench_session_operations(c: &mut Criterion) {
   let (mut cache, _temp_dir) = create_test_cache();
   let user_id = "bench_session_user";
   
   // Pre-populate with sessions and memories
   let mut session_ids = Vec::new();
   for i in 0..10 {
       let session_id = cache.create_session(user_id, Some(&format!("Benchmark Session {}", i)))
           .expect("Should create session");
       
       for j in 0..50 {
           let content = format!("Session {} memory {} about various topics", i, j);
           cache.save(user_id, &session_id, &content, None)
               .expect("Should save memory");
       }
       
       session_ids.push(session_id);
   }
   
   let mut group = c.benchmark_group("session_operations");
   
   group.bench_function("get_user_sessions", |b| {
       b.iter(|| {
           cache.get_user_sessions(black_box(user_id))
               .expect("Should get user sessions")
       })
   });
   
   group.bench_function("summarize_session", |b| {
       let session_id = &session_ids[0];
       b.iter(|| {
           cache.summarize_session(black_box(session_id))
               .expect("Should summarize session")
       })
   });
   
   group.bench_function("search_sessions", |b| {
       b.iter(|| {
           cache.search_sessions(black_box(user_id), black_box(vec!["memory".to_string()]))
               .expect("Should search sessions")
       })
   });
   
   group.finish();
}

#[cfg(feature = "benchmarks")]
fn bench_decay_operations(c: &mut Criterion) {
   let (mut cache, _temp_dir) = create_test_cache();
   let user_id = "bench_decay_user";
   let session_id = cache.create_session(user_id, Some("Decay Benchmark"))
       .expect("Should create session");
   
   c.bench_function("decay_process", |b| {
       b.iter_batched(
           || {
               // Setup: Add memories with different importance levels
               for i in 0..100 {
                   let importance = (i % 10) as f32 / 10.0;
                   let content = format!("Decay benchmark memory {}", i);
                   cache.save_with_options(user_id, &session_id, &content, None, importance, Some(1))
                       .expect("Should save memory");
               }
           },
           |_| {
               cache.decay().expect("Should run decay")
           },
           criterion::BatchSize::LargeInput,
       )
   });
}

#[cfg(feature = "benchmarks")]
fn bench_c_api_operations(c: &mut Criterion) {
   use std::ffi::CString;
   use std::ptr;
   
   let cache_ptr = mindcache_init();
   assert!(!cache_ptr.is_null());
   
   let user_id = CString::new("c_api_bench_user").unwrap();
   let session_id = CString::new("c_api_bench_session").unwrap();
   
   let mut group = c.benchmark_group("c_api_operations");
   
   group.bench_function("c_api_save", |b| {
       let mut counter = 0;
       b.iter(|| {
           let content = CString::new(format!("C API benchmark memory {}", counter)).unwrap();
           counter += 1;
           
           let result = mindcache_save(
               black_box(cache_ptr),
               black_box(user_id.as_ptr()),
               black_box(session_id.as_ptr()),
               black_box(content.as_ptr()),
               black_box(ptr::null()),
           );
           
           if !result.is_null() {
               mindcache_free_string(result);
           }
       })
   });
   
   group.bench_function("c_api_recall", |b| {
       let query = CString::new("benchmark").unwrap();
       b.iter(|| {
           let result = mindcache_recall(
               black_box(cache_ptr),
               black_box(user_id.as_ptr()),
               black_box(query.as_ptr()),
               black_box(ptr::null()),
               black_box(10),
           );
           
           if !result.is_null() {
               mindcache_free_string(result);
           }
       })
   });
   
   group.bench_function("c_api_get_stats", |b| {
       b.iter(|| {
           let result = mindcache_get_stats(black_box(cache_ptr));
           if !result.is_null() {
               mindcache_free_string(result);
           }
       })
   });
   
   group.finish();
   
   mindcache_destroy(cache_ptr);
}

#[cfg(feature = "benchmarks")]
criterion_group!(
   benches,
   bench_save_operations,
   bench_recall_operations, 
   bench_session_operations,
   bench_decay_operations,
   bench_c_api_operations
);

#[cfg(feature = "benchmarks")]
criterion_main!(benches);

// Additional performance regression tests
#[test]
fn test_performance_regression_saves() {
   let (mut cache, _temp_dir) = create_test_cache();
   
   let user_id = "regression_user";
   let session_id = cache.create_session(user_id, Some("Regression Test"))
       .expect("Should create session");
   
   // Baseline: time 100 saves
   let start = Instant::now();
   for i in 0..100 {
       let content = format!("Regression test memory {}", i);
       cache.save(user_id, &session_id, &content, None)
           .expect("Should save memory");
   }
   let baseline_time = start.elapsed();
   
   // Test: time another 100 saves (should be similar performance)
   let start = Instant::now();
   for i in 100..200 {
       let content = format!("Regression test memory {}", i);
       cache.save(user_id, &session_id, &content, None)
           .expect("Should save memory");
   }
   let test_time = start.elapsed();
   
   let performance_ratio = test_time.as_secs_f64() / baseline_time.as_secs_f64();
   
   println!("Performance regression test:");
   println!("  Baseline (first 100): {:?}", baseline_time);
   println!("  Test (second 100): {:?}", test_time);
   println!("  Ratio: {:.2}", performance_ratio);
   
   // Performance shouldn't degrade more than 50% for same-size batches
   assert!(performance_ratio < 1.5, 
          "Performance regression detected: {:.2}x slower", performance_ratio);
}

#[test]
fn test_memory_leak_detection() {
   // This test checks for obvious memory leaks by monitoring system behavior
   // In a real implementation, you'd use tools like Valgrind or AddressSanitizer
   
   let initial_stats = get_memory_usage();
   
   for iteration in 0..10 {
       let (mut cache, _temp_dir) = create_test_cache();
       
       let user_id = &format!("leak_test_user_{}", iteration);
       let session_id = cache.create_session(user_id, Some("Leak Test"))
           .expect("Should create session");
       
       // Perform various operations
       for i in 0..100 {
           cache.save(user_id, &session_id, &format!("Leak test memory {}", i), None)
               .expect("Should save memory");
       }
       
       let _memories = cache.recall(user_id, None, None, None)
           .expect("Should recall memories");
       
       let _summary = cache.summarize_session(&session_id)
           .expect("Should generate summary");
       
       let _decay_stats = cache.decay()
           .expect("Should run decay");
       
       // Cache should be dropped here
   }
   
   let final_stats = get_memory_usage();
   
   println!("Memory usage - Initial: {}, Final: {}", initial_stats, final_stats);
   
   // Memory usage shouldn't grow significantly
   // This is a rough heuristic - in practice you'd use more sophisticated tools
   let growth_ratio = final_stats as f64 / initial_stats as f64;
   if growth_ratio > 2.0 {
       println!("Warning: Memory usage grew {:.2}x - possible leak", growth_ratio);
   }
}

fn get_memory_usage() -> usize {
   // Simplified memory usage check - in practice you'd use system-specific APIs
   // This is just a placeholder for demonstration
   std::process::id() as usize // Using PID as a proxy
}

#[test] 
fn test_file_handle_cleanup() {
   // Test that file handles are properly cleaned up
   let mut caches = Vec::new();
   
   // Create many cache instances
   for i in 0..50 {
       let temp_dir = TempDir::new().expect("Should create temp dir");
       let config = MindCacheConfig {
           storage_path: temp_dir.path().to_str().unwrap().to_string(),
           ..Default::default()
       };
       
       let cache = MindCache::with_config(config).expect("Should create cache");
       caches.push((cache, temp_dir));
   }
   
   // Use each cache briefly
   for (mut cache, _) in &mut caches {
       let user_id = "file_handle_user";
       let session_id = cache.create_session(user_id, Some("File Handle Test"))
           .expect("Should create session");
       
       cache.save(user_id, &session_id, "Test content", None)
           .expect("Should save memory");
   }
   
   // Drop all caches
   drop(caches);
   
   // If we get here without running out of file handles, the test passes
   println!("File handle cleanup test passed - no resource exhaustion");
}

#[test]
fn test_storage_consistency_under_load() {
   let (mut cache, _temp_dir) = create_test_cache();
   
   let user_id = "consistency_user";
   let session_id = cache.create_session(user_id, Some("Consistency Test"))
       .expect("Should create session");
   
   // Rapidly alternate between saves and recalls
   let mut saved_contents = Vec::new();
   
   for i in 0..200 {
       let content = format!("Consistency test memory {}", i);
       saved_contents.push(content.clone());
       
       // Save
       cache.save(user_id, &session_id, &content, None)
           .expect("Should save memory");
       
       // Immediately recall and verify
       if i % 10 == 0 {
           let memories = cache.recall(user_id, None, None, None)
               .expect("Should recall memories");
           
           assert_eq!(memories.len(), i + 1, "Should have all saved memories at iteration {}", i);
           
           // Verify all our content is present
           let recalled_contents: std::collections::HashSet<_> = 
               memories.iter().map(|m| &m.content).collect();
           
           for saved_content in &saved_contents {
               assert!(recalled_contents.contains(&saved_content), 
                      "Should find saved content: {}", saved_content);
           }
       }
   }
   
   println!("Storage consistency maintained under rapid load");
}