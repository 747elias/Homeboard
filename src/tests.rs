use super::*;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use tower::ServiceExt;
fn app(dir: &FsPath) -> Arc<App> {
    Arc::new(App {
        dir: dir.into(),
        config: RwLock::new(config::load(dir).unwrap()),
        tasks: RwLock::new(tasks::load(dir).unwrap()),
        auth: Mutex::new(auth::AuthFile::default()),
        sessions: Mutex::new(auth::Sessions::default()),
        limiter: Mutex::new(auth::Limiter::default()),
        hashing: Semaphore::new(2),
        neighbors: Mutex::new(HashMap::new()),
        weather: weather::Weather::default(),
        client: reqwest::Client::new(),
        embeds: Mutex::new(HashMap::new()),
    })
}
async fn request(
    app: Arc<App>,
    method: &str,
    path: &str,
    ip: &str,
    body: String,
    cookie: &str,
    csrf: &str,
) -> Response {
    let req = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "localhost")
        .header("origin", "http://localhost")
        .header("content-type", "application/json")
        .header("cookie", cookie)
        .header("x-csrf-token", csrf)
        .extension(ConnectInfo(
            format!("{ip}:45678").parse::<SocketAddr>().unwrap(),
        ))
        .body(Body::from(body))
        .unwrap();
    router(app).oneshot(req).await.unwrap()
}
#[tokio::test]
async fn access_headers_login_csrf_save_and_logout() {
    let dir = tempfile::tempdir().unwrap();
    let a = app(dir.path());
    let denied = request(a.clone(), "GET", "/", "192.0.2.1", String::new(), "", "").await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let bytes = to_bytes(denied.into_body(), 1_048_576).await.unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("Dieser Raum ist privat"));
    assert_eq!(
        request(
            a.clone(),
            "GET",
            "/assets/denied.js",
            "192.0.2.1",
            String::new(),
            "",
            ""
        )
        .await
        .status(),
        StatusCode::OK
    );
    let editor = request(
        a.clone(),
        "GET",
        "/edit",
        "127.0.0.1",
        String::new(),
        "",
        "",
    )
    .await;
    assert_eq!(editor.headers()["x-frame-options"], "DENY");
    assert!(editor.headers().contains_key("content-security-policy"));
    let sandbox = request(
        a.clone(),
        "GET",
        "/sandbox",
        "127.0.0.1",
        String::new(),
        "",
        "",
    )
    .await;
    assert!(sandbox.headers()["content-security-policy"]
        .to_str()
        .unwrap()
        .contains("sandbox allow-scripts"));
    assert_eq!(
        request(
            a.clone(),
            "GET",
            "/api/editor/config",
            "127.0.0.1",
            String::new(),
            "",
            ""
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    let login = request(
        a.clone(),
        "POST",
        "/api/auth/login",
        "127.0.0.1",
        r#"{"password":"Test-Passwort-12345"}"#.into(),
        "",
        "",
    )
    .await;
    assert_eq!(login.status(), StatusCode::OK);
    let cookie = login.headers()["set-cookie"].to_str().unwrap().to_owned();
    assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"));
    let v: Value =
        serde_json::from_slice(&to_bytes(login.into_body(), 10000).await.unwrap()).unwrap();
    let token = v["csrf"].as_str().unwrap();
    assert_eq!(
        request(
            a.clone(),
            "GET",
            "/api/editor/weather?latitude=91&longitude=0",
            "127.0.0.1",
            String::new(),
            &cookie,
            ""
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    let mut c = a.config.read().await.clone();
    c.widgets[0].title = "Geändert".into();
    let body = serde_json::to_string(&c).unwrap();
    assert_eq!(
        request(
            a.clone(),
            "PUT",
            "/api/editor/config",
            "127.0.0.1",
            body.clone(),
            &cookie,
            ""
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            a.clone(),
            "PUT",
            "/api/editor/config",
            "127.0.0.1",
            body.clone(),
            &cookie,
            token
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        config::load(dir.path()).unwrap().widgets[0].title,
        "Geändert"
    );
    assert_eq!(
        request(
            a.clone(),
            "PUT",
            "/api/editor/config",
            "127.0.0.1",
            body,
            &cookie,
            token
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(
            a.clone(),
            "POST",
            "/api/auth/logout",
            "127.0.0.1",
            "{}".into(),
            &cookie,
            token
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(
            a,
            "GET",
            "/api/editor/config",
            "127.0.0.1",
            String::new(),
            &cookie,
            ""
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
}
#[tokio::test]
async fn first_setup_cannot_be_claimed_from_lan() {
    let dir = tempfile::tempdir().unwrap();
    let a = app(dir.path());
    {
        let mut c = a.config.write().await;
        c.access.entries.push("192.0.2.2".into());
        c.access.editor_local_only = false;
    }
    assert_eq!(
        request(
            a,
            "POST",
            "/api/auth/login",
            "192.0.2.2",
            r#"{"password":"Test-Passwort-12345"}"#.into(),
            "",
            ""
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
}
#[test]
fn dns_rebinding_hosts_and_origin() {
    assert!(valid_host("localhost:80"));
    assert!(valid_host("[::1]:80"));
    assert!(valid_host("192.168.1.2"));
    assert!(!valid_host("evil.example"));
    assert!(!valid_host("localhost@evil.example"));
    assert!(!valid_host("localhost/path"));
    let mut h = HeaderMap::new();
    h.insert("host", HeaderValue::from_static("localhost"));
    h.insert("origin", HeaderValue::from_static("http://evil.example"));
    assert!(!same_origin(&h));
    h.insert("origin", HeaderValue::from_static("http://localhost"));
    assert!(same_origin(&h));
}

#[tokio::test]
async fn tasks_require_session_csrf_revision_and_obey_local_editor_policy() {
    let dir = tempfile::tempdir().unwrap();
    let a = app(dir.path());
    let mut d = a.tasks.read().await.clone();
    d.tasks.push(tasks::Task {
        id: "api-task".into(),
        title: "Lokal planen".into(),
        ..tasks::Task::default()
    });
    let body = serde_json::to_string(&d).unwrap();
    assert_eq!(
        request(
            a.clone(),
            "PUT",
            "/api/tasks",
            "127.0.0.1",
            body.clone(),
            "",
            ""
        )
        .await
        .status(),
        StatusCode::UNAUTHORIZED
    );
    let login = request(
        a.clone(),
        "POST",
        "/api/auth/login",
        "127.0.0.1",
        r#"{"password":"Task-Test-123456"}"#.into(),
        "",
        "",
    )
    .await;
    let cookie = login.headers()["set-cookie"].to_str().unwrap().to_owned();
    let v: Value =
        serde_json::from_slice(&to_bytes(login.into_body(), 10000).await.unwrap()).unwrap();
    let csrf = v["csrf"].as_str().unwrap();
    assert_eq!(
        request(
            a.clone(),
            "PUT",
            "/api/tasks",
            "127.0.0.1",
            body.clone(),
            &cookie,
            ""
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            a.clone(),
            "PUT",
            "/api/tasks",
            "127.0.0.1",
            body.clone(),
            &cookie,
            csrf
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        tasks::load(dir.path()).unwrap().tasks[0].title,
        "Lokal planen"
    );
    assert_eq!(
        request(
            a.clone(),
            "PUT",
            "/api/tasks",
            "127.0.0.1",
            body,
            &cookie,
            csrf
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );
    a.config
        .write()
        .await
        .access
        .entries
        .push("192.0.2.22".into());
    assert_eq!(
        request(
            a.clone(),
            "GET",
            "/api/tasks",
            "192.0.2.22",
            String::new(),
            "",
            ""
        )
        .await
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(
            a.clone(),
            "PUT",
            "/api/tasks",
            "192.0.2.22",
            "{}".into(),
            "",
            ""
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            a.clone(),
            "GET",
            "/tasks",
            "192.0.2.99",
            String::new(),
            "",
            ""
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    let status = request(
        a,
        "GET",
        "/api/tasks/session",
        "192.0.2.22",
        String::new(),
        "",
        "",
    )
    .await;
    let v: Value =
        serde_json::from_slice(&to_bytes(status.into_body(), 10000).await.unwrap()).unwrap();
    assert_eq!(v["can_login"], false);
    assert!(v["csrf"].is_null());
}
