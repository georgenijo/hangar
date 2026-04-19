// Regression tests for #63: EventStore::insert on a deleted session must
// return Err with SQLite FK error code "787", not silently succeed or panic.
// This locks in the detection logic in the event persister (main.rs).

use hangard::{
    db::Db,
    events::{Event, EventStore},
    session::{Session, SessionId, SessionKind, SessionState},
};

fn make_session() -> Session {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    Session {
        id: SessionId::new(),
        slug: format!("fk-test-{}", ulid::Ulid::new()),
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
async fn event_for_deleted_session_does_not_warn() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let session = make_session();
    let id = session.id.to_string();
    session.insert(pool).await.unwrap();

    let deleted = Session::delete(pool, &session.id).await.unwrap();
    assert!(deleted, "session should have been deleted");

    let event = Event::SessionCreated;
    let result = EventStore::insert(pool, &id, &event).await;

    assert!(result.is_err(), "insert into deleted session must fail");
    let err = result.unwrap_err();
    let sqlx_err = err
        .downcast_ref::<sqlx::Error>()
        .expect("error should downcast to sqlx::Error");
    match sqlx_err {
        sqlx::Error::Database(dbe) => {
            assert_eq!(
                dbe.code().as_deref(),
                Some("787"),
                "expected SQLITE_CONSTRAINT_FOREIGNKEY (787)"
            );
        }
        other => panic!("expected Database error, got: {other:?}"),
    }
}

#[tokio::test]
async fn event_for_live_session_persists_ok() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let session = make_session();
    let id = session.id.to_string();
    session.insert(pool).await.unwrap();

    let event = Event::SessionCreated;
    let rowid = EventStore::insert(pool, &id, &event).await.unwrap();
    assert!(rowid > 0, "rowid should be positive");
}
