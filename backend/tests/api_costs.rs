use hangard::{
    db::Db,
    session::{AgentMeta, Session, SessionId, SessionKind, SessionState},
};

fn make_session_with_meta(
    pool: &sqlx::SqlitePool,
    meta: AgentMeta,
) -> impl std::future::Future<Output = Session> + '_ {
    async move {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;
        let s = Session {
            id: SessionId::new(),
            slug: format!("cost-test-{}", SessionId::new()),
            node_id: "local".to_string(),
            kind: SessionKind::ClaudeCode {
                config_override: None,
                project_dir: None,
            },
            state: SessionState::Exited,
            cwd: "/tmp".to_string(),
            env: serde_json::json!({}),
            agent_meta: None,
            labels: serde_json::json!([]),
            created_at: now,
            last_activity_at: now,
            exit: None,
            sandbox: None,
        };
        s.insert(pool).await.unwrap();
        Session::update_agent_meta(pool, &s.id, &meta).await.unwrap();
        s
    }
}

fn make_meta(model: &str, cost: f64, tokens: u64) -> AgentMeta {
    AgentMeta {
        name: "claude_code".to_string(),
        version: None,
        model: Some(model.to_string()),
        tokens_used: tokens,
        last_tool_call: None,
        context_pct: None,
        cost_dollars: Some(cost),
    }
}

#[tokio::test]
async fn costs_daily_empty_returns_empty_array() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    let rows = sqlx::query_as::<_, (String, f64, i64)>(
        r#"
        SELECT
            strftime('%Y-%m-%d', created_at / 1000, 'unixepoch') AS date,
            COALESCE(SUM(CAST(json_extract(agent_meta, '$.cost_dollars') AS REAL)), 0.0) AS usd,
            COALESCE(SUM(CAST(json_extract(agent_meta, '$.tokens_used') AS INTEGER)), 0) AS tokens
        FROM sessions
        WHERE agent_meta IS NOT NULL
        GROUP BY date
        ORDER BY date DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .unwrap();

    assert!(rows.is_empty(), "empty DB should yield no rows");
}

#[tokio::test]
async fn costs_daily_sums_sessions_by_day() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    make_session_with_meta(pool, make_meta("claude-sonnet-4", 0.10, 1000)).await;
    make_session_with_meta(pool, make_meta("claude-opus-4", 0.20, 2000)).await;

    let rows = sqlx::query_as::<_, (String, f64, i64)>(
        r#"
        SELECT
            strftime('%Y-%m-%d', created_at / 1000, 'unixepoch') AS date,
            COALESCE(SUM(CAST(json_extract(agent_meta, '$.cost_dollars') AS REAL)), 0.0) AS usd,
            COALESCE(SUM(CAST(json_extract(agent_meta, '$.tokens_used') AS INTEGER)), 0) AS tokens
        FROM sessions
        WHERE agent_meta IS NOT NULL
        GROUP BY date
        ORDER BY date DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 1, "both sessions on same day → 1 row");
    let (_, usd, tokens) = &rows[0];
    assert!((usd - 0.30).abs() < 1e-6, "total usd = 0.30, got {usd}");
    assert_eq!(*tokens, 3000, "total tokens = 3000");
}

#[tokio::test]
async fn costs_by_model_computes_share() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    make_session_with_meta(pool, make_meta("claude-sonnet-4", 0.10, 1000)).await;
    make_session_with_meta(pool, make_meta("claude-opus-4", 0.30, 2000)).await;

    let rows = sqlx::query_as::<_, (String, f64)>(
        r#"
        SELECT
            json_extract(agent_meta, '$.model') AS model,
            COALESCE(SUM(CAST(json_extract(agent_meta, '$.cost_dollars') AS REAL)), 0.0) AS usd
        FROM sessions
        WHERE agent_meta IS NOT NULL
          AND json_extract(agent_meta, '$.model') IS NOT NULL
          AND json_extract(agent_meta, '$.model') != 'null'
        GROUP BY model
        ORDER BY usd DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 2);
    let total: f64 = rows.iter().map(|r| r.1).sum();
    assert!((total - 0.40).abs() < 1e-6, "total = 0.40");

    let (model, usd) = &rows[0];
    assert_eq!(model, "claude-opus-4");
    let share = usd / total * 100.0;
    assert!((share - 75.0).abs() < 0.01, "opus share = 75%, got {share}");
}

#[tokio::test]
async fn costs_by_model_excludes_sessions_without_model() {
    let db = Db::new_in_memory().await.unwrap();
    let pool = db.pool();

    // Session with no model in agent_meta
    let meta_no_model = AgentMeta {
        name: "claude_code".to_string(),
        version: None,
        model: None,
        tokens_used: 500,
        last_tool_call: None,
        context_pct: None,
        cost_dollars: Some(0.05),
    };
    make_session_with_meta(pool, meta_no_model).await;
    make_session_with_meta(pool, make_meta("claude-sonnet-4", 0.10, 1000)).await;

    let rows = sqlx::query_as::<_, (String, f64)>(
        r#"
        SELECT json_extract(agent_meta, '$.model') AS model,
               COALESCE(SUM(CAST(json_extract(agent_meta, '$.cost_dollars') AS REAL)), 0.0) AS usd
        FROM sessions
        WHERE agent_meta IS NOT NULL
          AND json_extract(agent_meta, '$.model') IS NOT NULL
          AND json_extract(agent_meta, '$.model') != 'null'
        GROUP BY model
        ORDER BY usd DESC
        "#,
    )
    .fetch_all(pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 1, "only session with model should appear");
    assert_eq!(rows[0].0, "claude-sonnet-4");
}
