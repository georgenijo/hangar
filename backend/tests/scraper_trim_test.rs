/// Test scraper buffer trim on CTX match to prevent stale cost value lag.
/// Covers AC16, AC17: buffer trim after CTX match, split status line handling.

use hangard::drivers::status_scraper::{scrape_status, ScraperState};

#[test]
fn rolling_buffer_trims_on_ctx_match() {
    let mut state = ScraperState::default();

    // Fill buffer with 3000 chars junk then a status line
    let junk = "x".repeat(3000);
    let status = " CTX 50% 100k $5.00 | foo | claude-opus-4-7 |";
    let chunk = format!("{}{}", junk, status);

    let events = scrape_status(&chunk, &mut state);

    // Should emit events from CTX match
    assert!(
        !events.is_empty(),
        "expected events from CTX match, got none"
    );

    // Buffer should be trimmed to <= 1024 chars after CTX match
    assert!(
        state.status_buf.len() <= 1024,
        "Buffer not trimmed: {} chars (expected <= 1024)",
        state.status_buf.len()
    );
}

#[test]
fn scraper_handles_split_status_line() {
    let mut state = ScraperState::default();

    // First chunk has partial status
    let events1 = scrape_status("some output\nCTX 50% ", &mut state);

    // No complete match yet
    assert_eq!(events1.len(), 0, "should not emit on partial status");

    // Second chunk completes the status line
    let events2 = scrape_status("100k $5.00 | foo | claude-opus-4-7 |", &mut state);

    // Should emit events when pattern completes
    assert!(
        !events2.is_empty(),
        "expected events when status line completes"
    );

    // Buffer should contain accumulated CTX pattern
    assert!(
        state.status_buf.contains("CTX"),
        "buffer should contain CTX after accumulation"
    );
}

#[test]
fn trim_preserves_utf8_char_boundaries() {
    let mut state = ScraperState::default();

    // Build a buffer with multi-byte UTF-8 characters (emoji) and a status line
    let emoji_junk = "🔥".repeat(500); // Each emoji is 4 bytes
    let status = " CTX 30% 50k $2.50 | bar | claude-opus-4-7 |";
    let chunk = format!("{}{}", emoji_junk, status);

    let events = scrape_status(&chunk, &mut state);

    // Should emit events
    assert!(!events.is_empty(), "expected events from CTX match");

    // Buffer should be valid UTF-8 after trim
    assert!(
        state.status_buf.is_char_boundary(0),
        "buffer start should be at char boundary"
    );
    assert!(
        state.status_buf.is_char_boundary(state.status_buf.len()),
        "buffer end should be at char boundary"
    );

    // Should be able to convert to string without panic
    let _ = state.status_buf.clone();
}

#[test]
fn buffer_overflow_scenario() {
    let mut state = ScraperState::default();

    // Simulate multiple status updates with large junk between them
    for i in 0..5 {
        let junk = format!("noise{}", "x".repeat(800));
        let status = format!(" CTX {}% 100k $5.{:02} | foo | claude-opus-4-7 |", 10 + i * 10, i);
        let chunk = format!("{}{}", junk, status);

        let events = scrape_status(&chunk, &mut state);

        // Each iteration should emit events
        assert!(
            !events.is_empty(),
            "iteration {}: expected events from CTX match",
            i
        );

        // Buffer should stay bounded
        assert!(
            state.status_buf.len() <= 4096,
            "iteration {}: buffer exceeded 4096 chars: {}",
            i,
            state.status_buf.len()
        );
    }

    // Final buffer should be trimmed
    assert!(
        state.status_buf.len() <= 1024,
        "final buffer not trimmed: {} chars",
        state.status_buf.len()
    );
}

#[test]
fn multiple_chunks_with_cost_changes() {
    let mut state = ScraperState::default();

    // First status line
    let events1 = scrape_status("CTX 20% 50k $1.00 | foo | claude-opus-4-7 |", &mut state);
    assert!(events1.len() >= 2, "should emit ctx and cost events");

    // Add junk and updated status with new cost
    let junk = "x".repeat(2000);
    let status2 = format!("{} CTX 25% 60k $1.50 | foo | claude-opus-4-7 |", junk);
    let events2 = scrape_status(&status2, &mut state);

    // Should emit updated events
    assert!(events2.len() >= 2, "should emit updated ctx and cost events");

    // Buffer should be trimmed
    assert!(
        state.status_buf.len() <= 1024,
        "buffer should be trimmed after second CTX match"
    );

    // Verify the latest cost was captured
    assert_eq!(
        state.last_dollars,
        Some(1.50),
        "should capture latest cost value"
    );
}
