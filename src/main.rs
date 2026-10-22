mod access;
mod auth;
mod config;
#[cfg(windows)]
mod service;
mod weather;

use anyhow::{Context, Result};
use axum::{
    extract::{ConnectInfo, DefaultBodyLimit, Path, Query, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use config::{Config, ServerConfig};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    path::{Path as FsPath, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{watch, Mutex, RwLock, Semaphore};

#[derive(rust_embed::RustEmbed)]
#[folder = "web/"]
struct Assets;
type NeighborCache = HashMap<IpAddr, (Instant, Option<[u8; 6]>)>;

struct App {
    dir: PathBuf,
    config: RwLock<Config>,
    auth: Mutex<auth::AuthFile>,
    sessions: Mutex<auth::Sessions>,
    limiter: Mutex<auth::Limiter>,
    hashing: Semaphore,
    neighbors: Mutex<NeighborCache>,
    weather: weather::Weather,
    client: reqwest::Client,
    embeds: Mutex<HashMap<String, (Instant, EmbedStatus)>>,
}
#[derive(Clone)]
struct Peer(IpAddr);
#[derive(Debug)]
struct ApiError(StatusCode, &'static str);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error":self.1}))).into_response()
    }
}
type Api<T> = std::result::Result<T, ApiError>;
fn internal() -> ApiError {
    ApiError(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Aktion fehlgeschlagen. Daten und Berechtigungen prüfen.",
    )
}
fn bad(message: &'static str) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, message)
}
fn unauthorized() -> ApiError {
    ApiError(StatusCode::UNAUTHORIZED, "Bitte anmelden.")
}
fn sid(headers: &HeaderMap) -> &str {
    headers
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .split(';')
        .map(str::trim)
        .find_map(|v| v.strip_prefix("homeboard_session="))
        .unwrap_or("")
}
async fn session(app: &App, headers: &HeaderMap, ip: IpAddr) -> Api<String> {
    app.sessions
        .lock()
        .await
        .get(sid(headers), ip)
        .ok_or_else(unauthorized)
}
async fn csrf(app: &App, headers: &HeaderMap, ip: IpAddr) -> Api<()> {
    let token = session(app, headers, ip).await?;
    if headers.get("x-csrf-token").and_then(|v| v.to_str().ok()) != Some(&token) {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "Ungültiger CSRF-Schutz. Neu anmelden.",
        ));
    }
    Ok(())
}
fn valid_host(host: &str) -> bool {
    let Ok(u) = url::Url::parse(&format!("http://{host}")) else {
        return false;
    };
    u.username().is_empty()
        && u.password().is_none()
        && u.path() == "/"
        && u.query().is_none()
        && u.fragment().is_none()
        && u.host_str().is_some_and(|h| {
            h == "localhost" || h.trim_matches(['[', ']']).parse::<IpAddr>().is_ok()
        })
}
fn same_origin(headers: &HeaderMap) -> bool {
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) == Some(&format!("http://{host}"))
        && headers
            .get("sec-fetch-site")
            .and_then(|v| v.to_str().ok())
            .is_none_or(|v| v == "same-origin" || v == "none")
}
async fn guard(
    State(app): State<Arc<App>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    mut req: Request,
    next: Next,
) -> Response {
    let ip = access::normalize(peer.ip());
    let path = req.uri().path();
    let editor = path == "/edit"
        || path == "/edit/"
        || path == "/preview"
        || path.starts_with("/api/editor/")
        || path.starts_with("/api/auth/");
    let config = app.config.read().await;
    let mut allowed = access::allowed(ip, &config.access.entries, None);
    if !allowed
        && config
            .access
            .entries
            .iter()
            .any(|e| matches!(access::parse_entry(e), Ok(access::Entry::Mac(_))))
    {
        let mut neighbors = app.neighbors.lock().await;
        if neighbors.len() >= 1024 {
            neighbors.retain(|_, (t, _)| t.elapsed() < Duration::from_secs(30));
        }
        if neighbors.len() < 1024 {
            let cached = neighbors
                .get(&ip)
                .filter(|(t, _)| t.elapsed() < Duration::from_secs(30))
                .map(|(_, m)| *m);
            let mac = match cached {
                Some(m) => m,
                None => {
                    let m = access::neighbor(ip).await;
                    neighbors.insert(ip, (Instant::now(), m));
                    m
                }
            };
            allowed = access::allowed(ip, &config.access.entries, mac);
        }
    }
    if editor && config.access.editor_local_only && !ip.is_loopback() {
        allowed = false;
    }
    drop(config);
    if !allowed
        && matches!(
            path,
            "/assets/style.css" | "/assets/fonts.css" | "/assets/denied.js" | "/assets/logo.svg"
        )
    {
        return security_headers(next.run(req).await, false);
    }
    if !allowed {
        tracing::warn!(ip = %ip, "access_denied");
        return security_headers(asset("403.html"), editor);
    }
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !valid_host(host) {
        return security_headers(
            (
                StatusCode::BAD_REQUEST,
                "Bitte localhost oder die PC-IP verwenden.",
            )
                .into_response(),
            editor,
        );
    }
    if req.method() != "GET" && req.method() != "HEAD" && !same_origin(req.headers()) {
        return security_headers(
            ApiError(StatusCode::FORBIDDEN, "Anfrage muss von Homeboard stammen.").into_response(),
            editor,
        );
    }
    req.extensions_mut().insert(Peer(ip));
    security_headers(next.run(req).await, editor)
}
fn security_headers(mut response: Response, editor: bool) -> Response {
    let h = response.headers_mut();
    h.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    h.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    h.insert(
        "permissions-policy",
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    h.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    h.insert("content-security-policy", HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' https: http: data:; connect-src 'self'; frame-src 'self' https: http:; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'"));
    if h.remove("x-homeboard-sandbox").is_some() {
        h.insert("content-security-policy", HeaderValue::from_static("default-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'unsafe-inline'; img-src data: http: https:; connect-src 'none'; frame-src 'none'; object-src 'none'; base-uri 'none'; form-action 'none'; sandbox allow-scripts"));
    }
    if editor {
        h.insert("x-frame-options", HeaderValue::from_static("DENY"));
    }
    response
}
fn asset(name: &str) -> Response {
    match Assets::get(name) {
        Some(file) => {
            let status = if name == "403.html" {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::OK
            };
            (
                status,
                [(
                    header::CONTENT_TYPE,
                    mime_guess::from_path(name)
                        .first_or_octet_stream()
                        .as_ref()
                        .to_owned(),
                )],
                file.data.into_owned(),
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
async fn static_asset(Path(name): Path<String>) -> Response {
    asset(&name)
}
async fn dashboard(State(app): State<Arc<App>>) -> Json<Config> {
    let mut c = app.config.read().await.clone();
    c.access = config::Access::default();
    Json(c)
}
async fn editor_config(
    State(app): State<Arc<App>>,
    axum::Extension(Peer(ip)): axum::Extension<Peer>,
    headers: HeaderMap,
) -> Api<Json<Config>> {
    session(&app, &headers, ip).await?;
    Ok(Json(app.config.read().await.clone()))
}
async fn save_config(
    State(app): State<Arc<App>>,
    axum::Extension(Peer(ip)): axum::Extension<Peer>,
    headers: HeaderMap,
    Json(mut value): Json<Config>,
) -> Api<Json<Config>> {
    csrf(&app, &headers, ip).await?;
    value.validate().map_err(|_| {
        bad("Konfiguration ungültig. Widget-Werte, URLs, Darstellung und Whitelist prüfen.")
    })?;
    let mut current = app.config.write().await;
    if current.revision != value.revision {
        return Err(ApiError(StatusCode::CONFLICT, "Konfiguration wurde inzwischen geändert. Seite neu laden und Änderungen erneut anwenden."));
    }
    value.revision = current.revision.checked_add(1).ok_or_else(internal)?;
    let (dir, saved) = (app.dir.clone(), value.clone());
    tokio::task::spawn_blocking(move || config::save(&dir, &saved))
        .await
        .map_err(|_| internal())?
        .map_err(|_| internal())?;
    *current = value.clone();
    app.embeds.lock().await.clear();
    tracing::info!("configuration_saved");
    Ok(Json(value))
}
async fn status(
    State(app): State<Arc<App>>,
    axum::Extension(Peer(ip)): axum::Extension<Peer>,
    headers: HeaderMap,
) -> Json<Value> {
    let configured = app.auth.lock().await.password_hash.is_some();
    let token = session(&app, &headers, ip).await.ok();
    Json(
        json!({"configured": configured, "local":ip.is_loopback(), "authenticated": token.is_some(), "csrf":token}),
    )
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Credentials {
    password: String,
}
async fn authenticate(
    State(app): State<Arc<App>>,
    axum::Extension(Peer(ip)): axum::Extension<Peer>,
    Json(creds): Json<Credentials>,
) -> Api<Response> {
    if creds.password.len() > 1024 {
        return Err(bad("Passwort zu lang."));
    }
    if !app.limiter.lock().await.check(ip) {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "Zu viele Versuche. Bitte später erneut versuchen (bis zu 5 Minuten).",
        ));
    }
    let _permit = app
        .hashing
        .try_acquire()
        .map_err(|_| ApiError(StatusCode::TOO_MANY_REQUESTS, "Bitte kurz warten."))?;
    let mut authfile = app.auth.lock().await;
    let current = authfile.password_hash.clone();
    let first = current.is_none();
    if first && !ip.is_loopback() {
        return Err(ApiError(
            StatusCode::FORBIDDEN,
            "Erstes Passwort bitte direkt am PC unter localhost setzen.",
        ));
    }
    let password = creds.password;
    let result = tokio::task::spawn_blocking(move || -> Result<Option<String>> {
        if let Some(hash) = current {
            if auth::verify(&password, &hash) {
                Ok(None)
            } else {
                anyhow::bail!("login_failed")
            }
        } else {
            Ok(Some(auth::hash_password(&password)?))
        }
    })
    .await
    .map_err(|_| internal())?;
    let new_hash = match result {
        Ok(v) => v,
        Err(_) => {
            app.limiter.lock().await.failed(ip);
            return Err(unauthorized());
        }
    };
    if let Some(hash) = new_hash {
        let new = auth::AuthFile {
            password_hash: Some(hash),
        };
        new.save(&app.dir).map_err(|_| internal())?;
        *authfile = new;
    }
    drop(authfile);
    app.limiter.lock().await.success(ip);
    let (sid, csrf) = app.sessions.lock().await.create(ip);
    Ok((
        [(
            header::SET_COOKIE,
            format!("homeboard_session={sid}; HttpOnly; SameSite=Strict; Path=/; Max-Age=28800"),
        )],
        Json(json!({"csrf":csrf})),
    )
        .into_response())
}
async fn logout(
    State(app): State<Arc<App>>,
    axum::Extension(Peer(ip)): axum::Extension<Peer>,
    headers: HeaderMap,
) -> Api<Response> {
    csrf(&app, &headers, ip).await?;
    app.sessions.lock().await.0.remove(sid(&headers));
    Ok((
        [(
            header::SET_COOKIE,
            "homeboard_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0",
        )],
        Json(json!({"ok":true})),
    )
        .into_response())
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangePassword {
    current: String,
    password: String,
}
async fn change_password(
    State(app): State<Arc<App>>,
    axum::Extension(Peer(ip)): axum::Extension<Peer>,
    headers: HeaderMap,
    Json(value): Json<ChangePassword>,
) -> Api<Response> {
    csrf(&app, &headers, ip).await?;
    if value.current.len() > 1024 || value.password.len() > 1024 {
        return Err(bad("Passwort zu lang."));
    }
    if !app.limiter.lock().await.check(ip) {
        return Err(ApiError(
            StatusCode::TOO_MANY_REQUESTS,
            "Bitte später erneut versuchen.",
        ));
    }
    let _permit = app
        .hashing
        .try_acquire()
        .map_err(|_| ApiError(StatusCode::TOO_MANY_REQUESTS, "Bitte kurz warten."))?;
    let mut af = app.auth.lock().await;
    let hash = af.password_hash.clone().ok_or_else(unauthorized)?;
    let new = tokio::task::spawn_blocking(move || {
        if !auth::verify(&value.current, &hash) {
            anyhow::bail!("login_failed");
        }
        auth::hash_password(&value.password)
    })
    .await
    .map_err(|_| internal())?;
    let hash = match new {
        Ok(h) => h,
        Err(_) => {
            app.limiter.lock().await.failed(ip);
            return Err(bad(
                "Aktuelles Passwort falsch oder neues Passwort nicht 12–256 Zeichen lang.",
            ));
        }
    };
    let file = auth::AuthFile {
        password_hash: Some(hash),
    };
    file.save(&app.dir).map_err(|_| internal())?;
    *af = file;
    app.sessions.lock().await.0.clear();
    app.limiter.lock().await.success(ip);
    Ok((
        [(
            header::SET_COOKIE,
            "homeboard_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0",
        )],
        Json(json!({"ok":true})),
    )
        .into_response())
}
async fn forecast(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
) -> Api<Json<weather::CachedForecast>> {
    let c = app.config.read().await;
    let w = c
        .widgets
        .iter()
        .find(|w| w.id == id && w.kind == "weather")
        .ok_or_else(|| bad("Wetter-Widget nicht gefunden."))?;
    let (lat, lon) = (w.settings.latitude, w.settings.longitude);
    drop(c);
    app.weather
        .fetch(&app.client, lat, lon)
        .await
        .map(Json)
        .map_err(|_| {
            ApiError(
                StatusCode::BAD_GATEWAY,
                "Wetter gerade nicht erreichbar. Automatischer neuer Versuch folgt.",
            )
        })
}
#[derive(Deserialize)]
struct Search {
    name: String,
}
async fn geocode(
    State(app): State<Arc<App>>,
    axum::Extension(Peer(ip)): axum::Extension<Peer>,
    headers: HeaderMap,
    Query(q): Query<Search>,
) -> Api<Json<Value>> {
    session(&app, &headers, ip).await?;
    if q.name.chars().count() < 2 || q.name.len() > 200 {
        return Err(bad("Ortsname muss 2–200 Zeichen lang sein."));
    }
    let r = app
        .client
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query(&[
            ("name", q.name.as_str()),
            ("count", "8"),
            ("language", "de"),
            ("format", "json"),
        ])
        .send()
        .await
        .map_err(|_| ApiError(StatusCode::BAD_GATEWAY, "Ortssuche nicht erreichbar."))?
        .error_for_status()
        .map_err(|_| bad("Ortssuche fehlgeschlagen."))?;
    let value: Value = bounded_json(r)
        .await
        .map_err(|_| bad("Ungültige Ortsdaten."))?;
    Ok(Json(value))
}
#[derive(Clone, Serialize)]
struct EmbedStatus {
    allowed: bool,
    reason: String,
}
async fn embed_status(
    State(app): State<Arc<App>>,
    Path(id): Path<String>,
) -> Api<Json<EmbedStatus>> {
    let c = app.config.read().await;
    let w = c
        .widgets
        .iter()
        .find(|w| w.id == id && w.kind == "embed")
        .ok_or_else(|| bad("Einbettung nicht gefunden."))?;
    let s = w.settings.clone();
    drop(c);
    let fallback = |reason: &str| {
        Json(EmbedStatus {
            allowed: false,
            reason: reason.into(),
        })
    };
    if s.url.is_empty() {
        return Ok(fallback("Kalender-Link im Editor hinterlegen."));
    }
    let u = url::Url::parse(&s.url).map_err(|_| bad("Ungültige URL."))?;
    let host = u.host_str().unwrap_or("");
    if host == "tasks.google.com" || host == "keep.google.com" || s.embed_mode == "link" {
        return Ok(fallback("Diesen Dienst im eigenen Tab öffnen."));
    }
    if let Some((t, status)) = app.embeds.lock().await.get(&s.url) {
        if t.elapsed() < Duration::from_secs(900) {
            return Ok(Json(status.clone()));
        }
    }
    // Probe only a fixed, trusted endpoint family. Never fetch arbitrary widget URLs on the server.
    let trusted = u.scheme() == "https"
        && u.port_or_known_default() == Some(443)
        && matches!(host, "calendar.google.com" | "www.google.com")
        && u.path() == "/calendar/embed";
    if !trusted {
        return Ok(if s.embed_mode == "iframe" {
            Json(EmbedStatus {
                allowed: true,
                reason: "Manuell aktiviert. Falls der Anbieter blockiert, über Öffnen fortfahren."
                    .into(),
            })
        } else {
            fallback("Einbettung nicht verlässlich prüfbar. Sicher als Link öffnen oder im Editor bewusst iframe aktivieren.")
        });
    }
    let status = match app.client.get(u).send().await {
        Ok(response) if response.status().is_success() => {
            let xfo = response.headers().get("x-frame-options").is_some();
            let csp = response
                .headers()
                .get("content-security-policy")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            let restricted = csp.split(';').map(str::trim).any(|d| {
                d.starts_with("frame-ancestors")
                    && !d
                        .split_whitespace()
                        .skip(1)
                        .any(|v| matches!(v, "*" | "http:"))
            });
            EmbedStatus { allowed: !xfo && !restricted, reason: if xfo || restricted { "Der Anbieter blockiert diese Einbettung." } else { "Kalender-Einbettung verfügbar. Falls die Anmeldung blockiert wird, Öffnen verwenden." }.into() }
        }
        _ => EmbedStatus {
            allowed: false,
            reason: "Einbettung gerade nicht erreichbar oder Anmeldung erforderlich.".into(),
        },
    };
    let mut cache = app.embeds.lock().await;
    if cache.len() >= 100 {
        cache.clear();
    }
    cache.insert(s.url, (Instant::now(), status.clone()));
    Ok(Json(status))
}
pub async fn bounded_json<T: serde::de::DeserializeOwned>(
    mut response: reqwest::Response,
) -> Result<T> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len() + chunk.len() > 1_048_576 {
            anyhow::bail!("Antwort zu gross");
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&bytes)?)
}
fn router(app: Arc<App>) -> Router {
    Router::new()
        .route("/", get(|| async { asset("index.html") }))
        .route("/edit", get(|| async { asset("edit.html") }))
        .route("/edit/", get(|| async { asset("edit.html") }))
        .route("/preview", get(|| async { asset("index.html") }))
        .route(
            "/sandbox",
            get(|| async {
                let mut r = asset("sandbox.html");
                r.headers_mut()
                    .insert("x-homeboard-sandbox", HeaderValue::from_static("1"));
                r
            }),
        )
        .route("/assets/{*name}", get(static_asset))
        .route("/api/dashboard", get(dashboard))
        .route("/api/weather/{id}", get(forecast))
        .route("/api/embed/{id}", get(embed_status))
        .route("/api/auth/status", get(status))
        .route("/api/auth/login", post(authenticate))
        .route("/api/auth/logout", post(logout))
        .route("/api/editor/password", post(change_password))
        .route("/api/editor/config", get(editor_config).put(save_config))
        .route("/api/editor/geocode", get(geocode))
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                Html("<h1>Hier ist noch kein Zuhause.</h1><a href='/'>Zum Homeboard</a>"),
            )
        })
        .layer(DefaultBodyLimit::max(1_048_576))
        .layer(middleware::from_fn_with_state(app.clone(), guard))
        .with_state(app)
}
fn data_dir(args: &[String]) -> Result<PathBuf> {
    if let Some(i) = args.iter().position(|a| a == "--data-dir") {
        return args
            .get(i + 1)
            .map(PathBuf::from)
            .context("Pfad nach --data-dir fehlt");
    }
    if let Some(p) = std::env::var_os("HOMEBOARD_DATA_DIR") {
        return Ok(p.into());
    }
    #[cfg(windows)]
    {
        Ok(
            PathBuf::from(std::env::var_os("ProgramData").context("ProgramData fehlt")?)
                .join("Homeboard"),
        )
    }
    #[cfg(not(windows))]
    {
        Ok(PathBuf::from("data"))
    }
}
fn read_server(dir: &FsPath) -> Result<ServerConfig> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("server.json");
    if !path.exists() {
        config::atomic_write(&path, &serde_json::to_vec_pretty(&ServerConfig::default())?)?;
    }
    let value: ServerConfig = serde_json::from_slice(&std::fs::read(path)?)?;
    if value.port == 0 {
        anyhow::bail!("Port muss 1–65535 sein");
    }
    value
        .bind
        .parse::<IpAddr>()
        .context("Bind-Adresse muss eine IPv4- oder IPv6-Adresse sein")?;
    Ok(value)
}
fn bind(server: &ServerConfig) -> Result<std::net::TcpListener> {
    let ip: IpAddr = server.bind.parse()?;
    let sock = socket2::Socket::new(
        if ip.is_ipv6() {
            socket2::Domain::IPV6
        } else {
            socket2::Domain::IPV4
        },
        socket2::Type::STREAM,
        Some(socket2::Protocol::TCP),
    )?;
    if ip.is_ipv6() {
        sock.set_only_v6(false)?;
    }
    sock.set_nonblocking(true)?;
    sock.bind(&SocketAddr::new(ip, server.port).into()).with_context(|| format!("Port {} auf {} ist belegt oder nicht verfügbar. IIS/andere Server prüfen oder z. B. Port 8080 in server.json setzen.", server.port, server.bind))?;
    sock.listen(128)?;
    Ok(sock.into())
}
async fn run(dir: PathBuf, mut shutdown: watch::Receiver<bool>, console: bool) -> Result<()> {
    let server = read_server(&dir)?;
    let app = Arc::new(App {
        config: RwLock::new(config::load(&dir)?),
        auth: Mutex::new(auth::AuthFile::load(&dir)?),
        sessions: Mutex::new(auth::Sessions::default()),
        limiter: Mutex::new(auth::Limiter::default()),
        hashing: Semaphore::new(2),
        neighbors: Mutex::new(HashMap::new()),
        weather: weather::Weather::default(),
        client: reqwest::Client::builder()
            .timeout(Duration::from_secs(8))
            .connect_timeout(Duration::from_secs(4))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("Homeboard/0.1.0-beta")
            .build()?,
        embeds: Mutex::new(HashMap::new()),
        dir,
    });
    let listener = tokio::net::TcpListener::from_std(bind(&server)?)?;
    tracing::info!(port = server.port, "backend_started");
    axum::serve(
        listener,
        router(app).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        if console {
            tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = shutdown.changed() => {} }
        } else {
            let _ = shutdown.changed().await;
        }
    })
    .await?;
    tracing::info!("backend_stopped");
    Ok(())
}
fn logging(dir: &FsPath) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let logs = dir.join("logs");
    std::fs::create_dir_all(&logs)?;
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("homeboard")
        .filename_suffix("log")
        .max_log_files(14)
        .build(logs)?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_target(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(writer)
        .init();
    Ok(guard)
}
fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("Homeboard 0.1.0-beta\n--service  Windows-Dienst\n--check-port [--port 80]  Port prüfen (ohne Datenänderung)\n--reset-access  Whitelist zurücksetzen\n--reset-password  Admin-Passwort zurücksetzen\n--data-dir PFAD  Datenverzeichnis überschreiben");
        return Ok(());
    }
    if args.iter().any(|a| a == "--check-port") {
        let port = args
            .iter()
            .position(|a| a == "--port")
            .map(|i| {
                args.get(i + 1)
                    .context("Port fehlt")
                    .and_then(|p| p.parse().context("Ungültiger Port"))
            })
            .transpose()?
            .unwrap_or(80);
        if port == 0 {
            anyhow::bail!("Ungültiger Port");
        }
        let s = ServerConfig {
            port,
            ..ServerConfig::default()
        };
        let _ = bind(&s)?;
        return Ok(());
    }
    let dir = data_dir(&args)?;
    if args
        .iter()
        .any(|a| a == "--reset-access" || a == "--reset-password")
    {
        // Never run while the service is active: its later writes would overwrite this reset.
        let server = read_server(&dir)?;
        let _reservation =
            bind(&server).context("Backend zuerst beenden (Dienst Homeboard stoppen)")?;
        if args.iter().any(|a| a == "--reset-access") {
            let mut c = config::load(&dir)?;
            c.access = config::Access {
                entries: vec![],
                editor_local_only: true,
            };
            c.revision += 1;
            config::save(&dir, &c)?;
            println!("Zugriff zurückgesetzt. Nur localhost ist erlaubt.");
        }
        if args.iter().any(|a| a == "--reset-password") {
            auth::AuthFile::default().save(&dir)?;
            println!("Passwort zurückgesetzt. Neues Passwort unter http://localhost/edit setzen.");
        }
        return Ok(());
    }
    #[cfg(windows)]
    if args.iter().any(|a| a == "--service") {
        return service::dispatch();
    }
    let _guard = logging(&dir)?;
    let (_tx, rx) = watch::channel(false);
    let result = tokio::runtime::Runtime::new()?.block_on(run(dir, rx, true));
    if result.is_err() {
        tracing::error!("backend_start_failed_check_port_config_and_permissions");
    }
    result
}
#[cfg(test)]
mod tests;
