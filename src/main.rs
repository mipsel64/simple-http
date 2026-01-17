use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use clap::Parser;
use serde::Serialize;
use std::{
    borrow::Cow,
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};

type RequestCounts = Arc<RwLock<HashMap<String, u64>>>;

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
        let mut counts = state.counts.write().unwrap();
        *counts.entry(key).or_insert(0) += 1;
    }

    next.run(req).await
}

async fn counts(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    req: Request,
) -> impl IntoResponse {
    let counts = state.counts.read().unwrap().clone();

    let path = req.uri().path().to_string();
    let ip = get_addr(&req, &addr);
    let key = format!("{}:{}", ip, path);

    let num = counts.get(&key);
    Json(CountResponse {
        total: num.copied().unwrap_or_default(),
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
