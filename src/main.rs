use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use clap::Parser;
use serde::Serialize;
use std::{borrow::Cow, collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;

const DEFAULT_TTL: std::time::Duration = std::time::Duration::from_secs(15 * 60); // 15m

#[derive(Clone, Copy)]
struct Entry {
    value: u64,
    t0: std::time::Instant,
}

impl Default for Entry {
    fn default() -> Self {
        Self {
            value: 0,
            t0: std::time::Instant::now(),
        }
    }
}

impl Entry {
    fn inc(&mut self) {
        if self.should_reset() {
            self.reset();
        }
        self.value += 1;
    }

    fn get(&self) -> u64 {
        self.value
    }

    fn reset(&mut self) {
        self.value = 0;
        self.t0 = std::time::Instant::now();
    }

    fn should_reset(&self) -> bool {
        self.t0.elapsed() > DEFAULT_TTL
    }
}

type RequestCounts = Arc<RwLock<HashMap<String, Entry>>>;

#[derive(Clone)]
struct AppState {
    counts: RequestCounts,
}

#[derive(Serialize)]
struct CountResponse {
    total: u64,
    ip: String,
    path: String,
}

async fn count_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
    next: Next,
) -> Response {
    let path = req.uri().path().to_string();
    let ip = get_addr(&req, &addr);
    let key = format!("{}:{}", ip, path);
    {
        let mut counts = state.counts.write().await;
        let mut entry = *counts.entry(key).or_default();
        entry.inc();
    }

    next.run(req).await
}

async fn counts(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
) -> impl IntoResponse {
    let counts = state.counts.read().await;

    let path = req.uri().path().to_string();
    let ip = get_addr(&req, &addr);
    let key = format!("{}:{}", ip, path);

    let mut entry = counts.get(&key).copied().unwrap_or_default();

    if entry.should_reset() {
        let counts = state.counts.clone();
        tokio::spawn(async move {
            let mut write = counts.write().await;
            if let Some(entry) = write.get_mut(&key) {
                entry.reset();
            }
        });
        entry.reset();
        entry.inc();
    }

    Json(CountResponse {
        total: entry.get(),
        ip: ip.to_string(),
        path,
    })
}

async fn health() -> &'static str {
    "OK"
}

fn get_addr<'a>(req: &'a Request, peer: &SocketAddr) -> Cow<'a, str> {
    if let Some(hdr) = req.headers().get("cf-connecting-ip") {
        if let Ok(ip) = hdr.to_str() {
            if !ip.is_empty() {
                return Cow::Borrowed(ip);
            }
        }
    }
    Cow::Owned(peer.ip().to_string())
}

#[derive(clap::Parser)]
struct Command {
    #[clap(long, short, default_value = "0.0.0.0:8080")]
    address: String,
}

#[tokio::main]
async fn main() {
    let cmd = Command::parse();

    let state = AppState {
        counts: Arc::new(RwLock::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/healthz", get(health))
        .route("/*path", get(counts))
        .route("/", get(counts))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            count_middleware,
        ))
        .with_state(state);

    let addr: SocketAddr = cmd.address.parse().expect("Invalid address");
    println!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
