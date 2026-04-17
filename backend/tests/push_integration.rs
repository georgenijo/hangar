use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::Request;
use axum::response::IntoResponse;
use axum::routing::post;
use axum::Router;
use tokio::net::TcpListener;

#[derive(Clone, Default)]
struct Recorded {
    requests: Arc<Mutex<Vec<RecordedReq>>>,
}

#[derive(Debug)]
struct RecordedReq {
    body: String,
    title: Option<String>,
    priority: Option<String>,
    click: Option<String>,
}

async fn capture_handler(
    axum::extract::State(state): axum::extract::State<Recorded>,
    req: Request,
) -> impl IntoResponse {
    let title = req
        .headers()
        .get("x-title")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let priority = req
        .headers()
        .get("x-priority")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let click = req
        .headers()
        .get("x-click")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let body_bytes = axum::body::to_bytes(req.into_body(), usize::MAX)
        .await
        .unwrap_or_default();
    let body = String::from_utf8_lossy(&body_bytes).to_string();

    state.requests.lock().unwrap().push(RecordedReq {
        body,
        title,
        priority,
        click,
    });

    axum::http::StatusCode::OK
}

#[tokio::test]
async fn test_ntfy_client_sends_correct_headers() {
    let recorded = Recorded::default();
    let requests = recorded.requests.clone();

    let app = Router::new()
        .route("/hangar-test", post(capture_handler))
        .with_state(recorded);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let url = format!("http://{}", addr);
    let client = hangard::push::NtfyClient::new(&url, "hangar-test");

    client
        .send(hangard::push::Notification {
            title: "Test title".to_string(),
            body: "Test body".to_string(),
            priority: hangard::config::NtfyPriority::High,
            click_url: "https://example.com/session/abc".to_string(),
            tags: "bell".to_string(),
        })
        .await;

    tokio::time::sleep(Duration::from_millis(100)).await;

    let reqs = requests.lock().unwrap();
    assert_eq!(reqs.len(), 1, "expected one request");
    let r = &reqs[0];
    assert_eq!(r.body, "Test body");
    assert_eq!(r.title.as_deref(), Some("Test title"));
    assert_eq!(r.priority.as_deref(), Some("4"));
    assert_eq!(r.click.as_deref(), Some("https://example.com/session/abc"));
}

#[tokio::test]
async fn test_ntfy_client_tolerates_unreachable_server() {
    let client = hangard::push::NtfyClient::new("http://127.0.0.1:19999", "test");
    // Must not panic — errors are logged internally
    client
        .send(hangard::push::Notification {
            title: "Unreachable".to_string(),
            body: "Should not panic".to_string(),
            priority: hangard::config::NtfyPriority::Normal,
            click_url: "https://example.com/session/x".to_string(),
            tags: "warning".to_string(),
        })
        .await;
}
