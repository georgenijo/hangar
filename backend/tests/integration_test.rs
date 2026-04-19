use hangard::{
    db::Db,
    events::{AgentEvent, Event, EventStore},
    ringbuf::RingBuf,
    session::{Session, SessionId, SessionKind, SessionState},
};

fn make_session(id: &SessionId) -> Session {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    Session {
        id: id.clone(),
        slug: format!("test-{}", id),
        node_id: "local".to_string(),
        kind: SessionKind::Shell,
        state: SessionState::Idle,
        cwd: "/tmp".to_string(),
        env: serde_json::json!({}),
        agent_meta: None,
        labels: serde_json::json!({}),
        created_at: now,
        last_activity_at: now,
        exit: None,
        sandbox: None,
    }
}

#[tokio::test]
async fn test_write_2mb_wraps_ring_and_logs_events() {
    let dir = tempfile::tempdir().unwrap();
    let ring_path = dir.path().join("output.bin");

    let capacity: u64 = 1024 * 1024; // 1 MB
    let chunk_size: usize = 4096;
    let total_write: usize = 2 * 1024 * 1024; // 2 MB
    let num_chunks = total_write / chunk_size;

    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let session_id = SessionId::new();
    let session = make_session(&session_id);
    session.insert(pool).await.unwrap();

    let mut ring = RingBuf::create(&ring_path, capacity).unwrap();

    let mut written_offsets: Vec<(u64, u32)> = Vec::with_capacity(num_chunks);

    for i in 0..num_chunks {
        let byte_val = (i % 256) as u8;
        let chunk = vec![byte_val; chunk_size];
        let (offset, len) = ring.write(&chunk).unwrap();
        ring.sync().unwrap();

        let event = Event::OutputAppended {
            offset,
            len,
            text: None,
        };
        EventStore::insert(pool, session_id.as_ref(), &event)
            .await
            .unwrap();

        written_offsets.push((offset, len));
    }

    // head should be exactly 2 MB (monotonic, never wraps)
    assert_eq!(ring.head(), 2 * 1024 * 1024);

    // reopen and verify header persists
    drop(ring);
    let ring2 = RingBuf::open(&ring_path).unwrap();
    assert_eq!(ring2.head(), 2 * 1024 * 1024);
    assert_eq!(ring2.capacity(), capacity);

    // last 256 KB of writes should be readable and correct
    let check_from = num_chunks - 64; // last 64 chunks = 256 KB
    for (i, &(offset, len)) in written_offsets.iter().enumerate().skip(check_from) {
        let data = ring2.read_at(offset, len).unwrap();
        assert_eq!(data.len(), chunk_size);
        let expected_byte = (i % 256) as u8;
        assert!(
            data.iter().all(|&b| b == expected_byte),
            "chunk {} data mismatch at offset {}",
            i,
            offset
        );
    }

    // first writes are stale (ring wrapped twice — 2 MB written into 1 MB ring)
    let (stale_offset, stale_len) = written_offsets[0];
    let result = ring2.read_at(stale_offset, stale_len);
    assert!(result.is_err(), "expected stale read error for chunk 0");

    // verify events in DB
    let events = EventStore::query(pool, session_id.as_ref(), 0, Some("OutputAppended"), 1000)
        .await
        .unwrap();
    assert_eq!(events.len(), num_chunks.min(1000));

    // verify MessagePack round-trip and monotonic offsets
    let queried = EventStore::query(pool, session_id.as_ref(), 0, Some("OutputAppended"), 1000)
        .await
        .unwrap();

    for (i, stored) in queried.iter().enumerate() {
        let expected_offset = (i * chunk_size) as u64;
        match &stored.event {
            Event::OutputAppended { offset, len, .. } => {
                assert_eq!(*offset, expected_offset, "event {} offset mismatch", i);
                assert_eq!(*len, chunk_size as u32, "event {} len mismatch", i);
            }
            other => panic!("expected OutputAppended, got {:?}", other),
        }
    }

    // recent event offsets should resolve; old ones should fail
    let recent_event = &queried[queried.len() - 1];
    if let Event::OutputAppended { offset, len, .. } = &recent_event.event {
        let data = ring2.read_at(*offset, *len).unwrap();
        assert_eq!(data.len(), chunk_size);
    }

    let old_event = &queried[0];
    if let Event::OutputAppended { offset, len, .. } = &old_event.event {
        assert!(
            ring2.read_at(*offset, *len).is_err(),
            "expected stale error for oldest event"
        );
    }
}

#[tokio::test]
async fn test_session_claude_code_kind_round_trip() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let session_id = SessionId::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let session = Session {
        id: session_id.clone(),
        slug: format!("test-cc-{}", session_id),
        node_id: "local".to_string(),
        kind: SessionKind::ClaudeCode {
            config_override: None,
            project_dir: None,
        },
        state: SessionState::Idle,
        cwd: "/tmp".to_string(),
        env: serde_json::json!({}),
        agent_meta: None,
        labels: serde_json::json!({}),
        created_at: now,
        last_activity_at: now,
        exit: None,
        sandbox: None,
    };

    session.insert(pool).await.unwrap();
    let loaded = Session::get(pool, &session_id).await.unwrap().unwrap();

    assert!(matches!(
        loaded.kind,
        SessionKind::ClaudeCode {
            config_override: None,
            project_dir: None
        }
    ));
    assert_eq!(loaded.state, SessionState::Idle);
}

#[tokio::test]
async fn test_agent_event_round_trip() {
    use hangard::events::{AgentEvent, TurnRole};

    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let session_id = SessionId::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let session = Session {
        id: session_id.clone(),
        slug: format!("test-ae-{}", session_id),
        node_id: "local".to_string(),
        kind: SessionKind::Shell,
        state: SessionState::Idle,
        cwd: "/tmp".to_string(),
        env: serde_json::json!({}),
        agent_meta: None,
        labels: serde_json::json!({}),
        created_at: now,
        last_activity_at: now,
        exit: None,
        sandbox: None,
    };
    session.insert(pool).await.unwrap();

    let event = Event::AgentEvent {
        id: session_id.clone(),
        event: AgentEvent::TurnStarted {
            turn_id: 1,
            role: TurnRole::User,
            content_start: Some("hello".to_string()),
        },
    };

    EventStore::insert(pool, session_id.as_ref(), &event)
        .await
        .unwrap();

    let stored = EventStore::query(pool, session_id.as_ref(), 0, Some("AgentEvent"), 10)
        .await
        .unwrap();

    assert_eq!(stored.len(), 1);
    assert!(matches!(
        &stored[0].event,
        Event::AgentEvent {
            event: AgentEvent::TurnStarted { turn_id: 1, .. },
            ..
        }
    ));
}

#[tokio::test]
async fn test_fts_insert_and_search() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let sid = SessionId::new();
    make_session(&sid).insert(pool).await.unwrap();
    let sid_str = sid.to_string();

    EventStore::insert(
        pool,
        &sid_str,
        &Event::AgentEvent {
            id: sid.clone(),
            event: AgentEvent::ToolCallStarted {
                turn_id: 1,
                call_id: "c1".into(),
                tool: "Bash".into(),
                args_preview: "cargo build".into(),
            },
        },
    )
    .await
    .unwrap();

    EventStore::insert(
        pool,
        &sid_str,
        &Event::AgentEvent {
            id: sid.clone(),
            event: AgentEvent::Error {
                message: "compilation failed".into(),
            },
        },
    )
    .await
    .unwrap();

    EventStore::insert(
        pool,
        &sid_str,
        &Event::AgentEvent {
            id: sid.clone(),
            event: AgentEvent::ToolCallFinished {
                turn_id: 1,
                call_id: "c1".into(),
                ok: true,
                result_preview: "success".into(),
            },
        },
    )
    .await
    .unwrap();

    let results = EventStore::search(pool, "cargo", None, None, 10, 0)
        .await
        .unwrap();
    assert_eq!(results.len(), 1, "expected 1 result for 'cargo'");
    assert!(
        results[0].snippet.contains("<mark>"),
        "snippet should contain <mark>"
    );
}

#[tokio::test]
async fn test_fts_cross_session_search() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let sid1 = SessionId::new();
    let sid2 = SessionId::new();
    make_session(&sid1).insert(pool).await.unwrap();
    make_session(&sid2).insert(pool).await.unwrap();
    let s1 = sid1.to_string();
    let s2 = sid2.to_string();

    EventStore::insert(
        pool,
        &s1,
        &Event::AgentEvent {
            id: sid1.clone(),
            event: AgentEvent::Error {
                message: "deploy failed in session one".into(),
            },
        },
    )
    .await
    .unwrap();

    EventStore::insert(
        pool,
        &s2,
        &Event::AgentEvent {
            id: sid2.clone(),
            event: AgentEvent::Error {
                message: "deploy failed in session two".into(),
            },
        },
    )
    .await
    .unwrap();

    // Cross-session search — should return both
    let all = EventStore::search(pool, "deploy", None, None, 10, 0)
        .await
        .unwrap();
    assert_eq!(all.len(), 2, "expected 2 cross-session results");

    // Filtered to session 1 only
    let filtered = EventStore::search(pool, "deploy", Some(&[s1.as_str()]), None, 10, 0)
        .await
        .unwrap();
    assert_eq!(filtered.len(), 1, "expected 1 result for session 1");
    assert_eq!(filtered[0].session_id, s1);
}

#[tokio::test]
async fn test_fts_kind_filter() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let sid = SessionId::new();
    make_session(&sid).insert(pool).await.unwrap();
    let sid_str = sid.to_string();

    EventStore::insert(
        pool,
        &sid_str,
        &Event::AgentEvent {
            id: sid.clone(),
            event: AgentEvent::Error {
                message: "unique_keyword_xyz error".into(),
            },
        },
    )
    .await
    .unwrap();

    // StateChanged won't produce body_text, so only AgentEvent is indexed
    let agent_results = EventStore::search(
        pool,
        "unique_keyword_xyz",
        Some(&[]),
        Some(&["AgentEvent"]),
        10,
        0,
    )
    .await
    .unwrap();
    assert_eq!(agent_results.len(), 1);

    let other_results = EventStore::search(
        pool,
        "unique_keyword_xyz",
        Some(&[]),
        Some(&["StateChanged"]),
        10,
        0,
    )
    .await
    .unwrap();
    assert_eq!(other_results.len(), 0);
}

#[tokio::test]
async fn test_fts_pagination() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let sid = SessionId::new();
    make_session(&sid).insert(pool).await.unwrap();
    let sid_str = sid.to_string();

    for i in 0..15 {
        EventStore::insert(
            pool,
            &sid_str,
            &Event::AgentEvent {
                id: sid.clone(),
                event: AgentEvent::Error {
                    message: format!("pagination_test_event number {i}"),
                },
            },
        )
        .await
        .unwrap();
    }

    let page1 = EventStore::search(pool, "pagination_test_event", None, None, 5, 0)
        .await
        .unwrap();
    let page2 = EventStore::search(pool, "pagination_test_event", None, None, 5, 5)
        .await
        .unwrap();

    assert_eq!(page1.len(), 5);
    assert_eq!(page2.len(), 5);

    let ids1: std::collections::HashSet<i64> = page1.iter().map(|r| r.event_id).collect();
    let ids2: std::collections::HashSet<i64> = page2.iter().map(|r| r.event_id).collect();
    assert!(
        ids1.is_disjoint(&ids2),
        "pages should have non-overlapping event ids"
    );
}

#[tokio::test]
async fn test_fts_backfill() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let sid = SessionId::new();
    make_session(&sid).insert(pool).await.unwrap();
    let sid_str = sid.to_string();

    // Insert directly bypassing EventStore::insert to simulate pre-migration rows
    let event = Event::AgentEvent {
        id: sid.clone(),
        event: AgentEvent::Error {
            message: "backfill_search_term".into(),
        },
    };
    let body = rmp_serde::to_vec(&event).unwrap();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    sqlx::query("INSERT INTO events (session_id, ts, kind, body) VALUES (?, ?, ?, ?)")
        .bind(&sid_str)
        .bind(ts)
        .bind("AgentEvent")
        .bind(&body)
        .execute(pool)
        .await
        .unwrap();

    // Before backfill: search returns nothing
    let before = EventStore::search(pool, "backfill_search_term", None, None, 10, 0)
        .await
        .unwrap();
    assert_eq!(before.len(), 0, "no results before backfill");

    // Run backfill
    let count = EventStore::backfill_fts(pool).await.unwrap();
    assert_eq!(count, 1, "expected 1 event backfilled");

    // After backfill: search finds the event
    let after = EventStore::search(pool, "backfill_search_term", None, None, 10, 0)
        .await
        .unwrap();
    assert_eq!(after.len(), 1, "expected 1 result after backfill");
}

#[tokio::test]
async fn test_fts_malformed_query() {
    use hangard::events::SearchError;

    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    // Formerly-malformed queries are now escaped into valid FTS5 phrase searches,
    // so they return Ok([]) rather than an error. Either outcome is acceptable;
    // what must never happen is a panic or SearchError::Db (500).
    let result = EventStore::search(pool, "unbalanced \"quote", None, None, 10, 0).await;
    match result {
        Ok(_) => {}
        Err(SearchError::BadQuery(_)) => {}
        Err(SearchError::Db(e)) => panic!("query caused Db error (500): {e}"),
    }
}

#[tokio::test]
async fn test_extract_searchable_text_all_variants() {
    use hangard::events::{extract_searchable_text_pub, AgentEvent, Event, TurnRole};

    let sid = SessionId::new();

    // TurnStarted with content
    assert!(extract_searchable_text_pub(&Event::AgentEvent {
        id: sid.clone(),
        event: AgentEvent::TurnStarted {
            turn_id: 1,
            role: TurnRole::User,
            content_start: Some("hello".into()),
        },
    })
    .is_some());

    // TurnStarted without content
    assert!(extract_searchable_text_pub(&Event::AgentEvent {
        id: sid.clone(),
        event: AgentEvent::TurnStarted {
            turn_id: 1,
            role: TurnRole::User,
            content_start: None,
        },
    })
    .is_none());

    // TurnFinished
    assert!(extract_searchable_text_pub(&Event::AgentEvent {
        id: sid.clone(),
        event: AgentEvent::TurnFinished {
            turn_id: 1,
            tokens_used: 100,
            duration_ms: 500,
        },
    })
    .is_none());

    // ThinkingBlock
    assert!(extract_searchable_text_pub(&Event::AgentEvent {
        id: sid.clone(),
        event: AgentEvent::ThinkingBlock {
            turn_id: 1,
            len_chars: 200,
        },
    })
    .is_none());

    // ToolCallStarted
    assert!(extract_searchable_text_pub(&Event::AgentEvent {
        id: sid.clone(),
        event: AgentEvent::ToolCallStarted {
            turn_id: 1,
            call_id: "c1".into(),
            tool: "Bash".into(),
            args_preview: "ls".into(),
        },
    })
    .is_some());

    // ToolCallFinished with content
    assert!(extract_searchable_text_pub(&Event::AgentEvent {
        id: sid.clone(),
        event: AgentEvent::ToolCallFinished {
            turn_id: 1,
            call_id: "c1".into(),
            ok: true,
            result_preview: "done".into(),
        },
    })
    .is_some());

    // ToolCallFinished empty
    assert!(extract_searchable_text_pub(&Event::AgentEvent {
        id: sid.clone(),
        event: AgentEvent::ToolCallFinished {
            turn_id: 1,
            call_id: "c1".into(),
            ok: true,
            result_preview: "".into(),
        },
    })
    .is_none());

    // AwaitingPermission
    assert!(extract_searchable_text_pub(&Event::AgentEvent {
        id: sid.clone(),
        event: AgentEvent::AwaitingPermission {
            tool: "Bash".into(),
            prompt: "allow?".into(),
        },
    })
    .is_some());

    // ModelChanged
    assert!(extract_searchable_text_pub(&Event::AgentEvent {
        id: sid.clone(),
        event: AgentEvent::ModelChanged {
            model: "claude-3".into(),
        },
    })
    .is_some());

    // Error
    assert!(extract_searchable_text_pub(&Event::AgentEvent {
        id: sid.clone(),
        event: AgentEvent::Error {
            message: "oops".into(),
        },
    })
    .is_some());

    // ContextWindowSizeChanged
    assert!(extract_searchable_text_pub(&Event::AgentEvent {
        id: sid.clone(),
        event: AgentEvent::ContextWindowSizeChanged {
            pct_used: 50.0,
            tokens: 1000,
        },
    })
    .is_none());

    // Non-agent variants
    assert!(extract_searchable_text_pub(&Event::SessionCreated).is_none());
    assert!(extract_searchable_text_pub(&Event::MetricsUpdated).is_none());
    assert!(extract_searchable_text_pub(&Event::OutputAppended {
        offset: 0,
        len: 100,
        text: None,
    })
    .is_none());
    assert!(extract_searchable_text_pub(&Event::OutputAppended {
        offset: 0,
        len: 100,
        text: Some("hello".into()),
    })
    .is_some());
    assert!(extract_searchable_text_pub(&Event::InputReceived { data: vec![] }).is_none());
    assert!(extract_searchable_text_pub(&Event::InputReceived {
        data: b"abc".to_vec()
    })
    .is_some());
    assert!(extract_searchable_text_pub(&Event::Resized { cols: 80, rows: 24 }).is_none());
    assert!(extract_searchable_text_pub(&Event::StateChanged {
        from: hangard::session::SessionState::Idle,
        to: hangard::session::SessionState::Exited,
    })
    .is_none());
}

#[tokio::test]
async fn test_fts_hyphen_query_no_500() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let sid = SessionId::new();
    make_session(&sid).insert(pool).await.unwrap();
    let sid_str = sid.to_string();

    EventStore::insert(
        pool,
        &sid_str,
        &Event::AgentEvent {
            id: sid.clone(),
            event: AgentEvent::ToolCallStarted {
                turn_id: 1,
                call_id: "c1".into(),
                tool: "Bash".into(),
                args_preview: "dash-delimited slug".into(),
            },
        },
    )
    .await
    .unwrap();

    let result = EventStore::search(pool, "dash-delimited", None, None, 10, 0).await;
    assert!(
        result.is_ok(),
        "hyphen query should not error: {:?}",
        result
    );
    assert!(
        !result.unwrap().is_empty(),
        "expected at least 1 result for dash-delimited"
    );
}

#[tokio::test]
async fn test_fts_colon_query_no_500() {
    use hangard::events::SearchError;

    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let sid = SessionId::new();
    make_session(&sid).insert(pool).await.unwrap();
    let sid_str = sid.to_string();

    EventStore::insert(
        pool,
        &sid_str,
        &Event::AgentEvent {
            id: sid.clone(),
            event: AgentEvent::Error {
                message: "foo:bar baz".into(),
            },
        },
    )
    .await
    .unwrap();

    let result = EventStore::search(pool, "foo:bar", None, None, 10, 0).await;
    match &result {
        Ok(_) => {}
        Err(SearchError::BadQuery(_)) => {}
        Err(SearchError::Db(e)) => panic!("colon query caused Db error (500): {e}"),
    }
}

#[tokio::test]
async fn test_fts_special_chars_no_db_error() {
    use hangard::events::SearchError;

    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    for q in &["*abc", "(a|b)", "a\"b", "-"] {
        let result = EventStore::search(pool, q, None, None, 10, 0).await;
        match &result {
            Ok(_) => {}
            Err(SearchError::BadQuery(_)) => {}
            Err(SearchError::Db(e)) => {
                panic!("query {:?} caused Db error (500): {e}", q)
            }
        }
    }
}

#[tokio::test]
async fn test_shell_output_indexed() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let sid = SessionId::new();
    make_session(&sid).insert(pool).await.unwrap();
    let sid_str = sid.to_string();

    EventStore::insert(
        pool,
        &sid_str,
        &Event::OutputAppended {
            offset: 0,
            len: 26,
            text: Some("hello dash-delimited world".into()),
        },
    )
    .await
    .unwrap();

    let results = EventStore::search(pool, "dash-delimited", None, None, 10, 0)
        .await
        .unwrap();
    assert_eq!(
        results.len(),
        1,
        "expected 1 result for shell output search"
    );
    assert_eq!(results[0].kind, "OutputAppended");
    assert!(
        results[0].snippet.contains("dash"),
        "snippet should contain matched text"
    );
}

#[tokio::test]
async fn test_shell_input_indexed() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let sid = SessionId::new();
    make_session(&sid).insert(pool).await.unwrap();
    let sid_str = sid.to_string();

    EventStore::insert(
        pool,
        &sid_str,
        &Event::InputReceived {
            data: b"grep foo:bar /tmp\n".to_vec(),
        },
    )
    .await
    .unwrap();

    let results = EventStore::search(pool, "foo:bar", None, None, 10, 0)
        .await
        .unwrap();
    assert!(
        !results.is_empty(),
        "expected at least 1 result for input search"
    );
}

#[tokio::test]
async fn test_shell_output_ansi_stripped() {
    use hangard::pty::indexable_text_from_chunk;

    let chunk = b"\x1b[31mred\x1b[0m text";
    let result = indexable_text_from_chunk(chunk);
    assert!(result.is_some(), "non-empty visible text should be indexed");
    let text = result.unwrap();
    assert!(
        !text.contains('\x1b'),
        "stored text should not contain ANSI escape bytes"
    );
    assert!(text.contains("red"), "text should contain 'red'");
    assert!(text.contains("text"), "text should contain 'text'");
}
