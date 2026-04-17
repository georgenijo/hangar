use hangard::{
    db::Db,
    events::{Event, EventStore},
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
        state: SessionState::Running,
        cwd: "/tmp".to_string(),
        env: serde_json::json!({}),
        agent_meta: None,
        labels: serde_json::json!({}),
        created_at: now,
        last_activity_at: now,
        exit: None,
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

        let event = Event::OutputAppended { offset, len };
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
    for i in check_from..num_chunks {
        let (offset, len) = written_offsets[i];
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
            Event::OutputAppended { offset, len } => {
                assert_eq!(*offset, expected_offset, "event {} offset mismatch", i);
                assert_eq!(*len, chunk_size as u32, "event {} len mismatch", i);
            }
            other => panic!("expected OutputAppended, got {:?}", other),
        }
    }

    // recent event offsets should resolve; old ones should fail
    let recent_event = &queried[queried.len() - 1];
    if let Event::OutputAppended { offset, len } = &recent_event.event {
        let data = ring2.read_at(*offset, *len).unwrap();
        assert_eq!(data.len(), chunk_size);
    }

    let old_event = &queried[0];
    if let Event::OutputAppended { offset, len } = &old_event.event {
        assert!(
            ring2.read_at(*offset, *len).is_err(),
            "expected stale error for oldest event"
        );
    }
}
