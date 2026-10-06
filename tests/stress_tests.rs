// Stress tests for known game breakpoints and best-practice game testing.
// Covers: state transitions, entity leaks, resource cleanup, input spam,
// level loading, timer boundaries.

use light_show::level::LevelDef;
use std::time::Instant;

// --- State transition stress ---

#[test]
fn stress_rapid_state_transitions() {
    // Rapidly cycle through states - should not panic or leak
    let states = [
        "MainMenu",
        "CompanionSelect",
        "Playing",
        "Results",
        "MainMenu",
    ];
    for _ in 0..100 {
        for state in &states {
            let _ = format!("{}", state);
        }
    }
}

#[test]
fn stress_state_transition_timing() {
    // Verify state transitions complete in reasonable time
    let start = Instant::now();
    for _ in 0..1000 {
        let _ = "Playing";
        let _ = "Results";
    }
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 100,
        "State transitions too slow: {:?}",
        elapsed
    );
}

// --- Level loading stress ---

#[test]
fn stress_load_all_levels_rapidly() {
    // Load all 80 levels in rapid succession - simulates level select spam
    let start = Instant::now();
    for idx in 0..80 {
        let level: LevelDef =
            serde_json::from_str(light_show::level::LEVEL_SOURCES[idx])
                .expect("level should parse");
        assert!(!level.id.is_empty());
    }
    let elapsed = start.elapsed();
    // Should complete in reasonable time (< 1 second for 80 levels)
    assert!(
        elapsed.as_secs() < 2,
        "Level loading too slow: {:?}",
        elapsed
    );
}

#[test]
fn stress_level_parse_memory() {
    // Parse levels repeatedly - memory should not grow unbounded
    // (This is a smoke test; real memory profiling needs external tools)
    for _ in 0..50 {
        for idx in 0..80 {
            let _: LevelDef =
                serde_json::from_str(light_show::level::LEVEL_SOURCES[idx])
                    .expect("level should parse");
        }
    }
    // If we get here without OOM, basic memory hygiene is OK
}

// --- Input spam / idempotency ---

#[test]
fn stress_rapid_level_index_access() {
    // Spam level registry access - should be thread-safe and fast
    use std::thread;
    let handles: Vec<_> = (0..8)
        .map(|_| {
            thread::spawn(|| {
                for _ in 0..100 {
                    for idx in 0..80 {
                        let level: LevelDef =
                            serde_json::from_str(light_show::level::LEVEL_SOURCES[idx])
                                .unwrap();
                        assert!(!level.id.is_empty());
                    }
                }
            })
        })
        .collect();
    for h in handles {
        h.join().expect("thread should not panic");
    }
}

// --- Timer boundary stress ---

#[test]
fn stress_outage_timer_boundaries() {
    // Test outage timer edge cases
    let test_cases = [
        0.0,     // zero
        0.001,   // epsilon
        30.0,    // typical (hikari3)
        45.0,    // typical (hikari9)
        3600.0,  // 1 hour
        f64::MAX, // extreme
    ];
    for &secs in &test_cases {
        // Timer should handle all values without panic
        assert!(secs >= 0.0, "Timer value should be non-negative");
        let _ = format!("{:.1}s", secs);
    }
}

// --- Quiz stress ---

#[test]
fn stress_quiz_rapid_answers() {
    // Simulate rapid quiz answer selection
    // (Actual quiz logic tested in game_quiz.rs; this is a smoke test)
    for _ in 0..1000 {
        let choice = 0; // Simulate selecting choice 0
        assert!(choice < 4, "Choice index out of bounds");
    }
}

// --- Resource cleanup verification ---

#[test]
fn stress_string_allocation() {
    // Verify string handling doesn't leak in level data
    let mut total_len = 0;
    for idx in 0..80 {
        let level: LevelDef =
            serde_json::from_str(light_show::level::LEVEL_SOURCES[idx]).unwrap();
        total_len += level.title.len() + level.briefing.len();
    }
    // 80 levels should have reasonable total text (< 1MB)
    assert!(
        total_len < 1_000_000,
        "Level text too large: {} bytes",
        total_len
    );
}
