mod pager;
mod node;
mod btree;
mod b_minus_node;
mod b_minus_tree;

use axum::{
    routing::{get, post},
    Router, Json, extract::State,
};
use std::sync::{Arc, Mutex};
use tower_http::cors::{CorsLayer, Any};
use serde::Deserialize;

use pager::Pager;
use btree::BTree;
use b_minus_tree::BMinusTree;

#[derive(Clone)]
struct AppState {
    btree: Arc<Mutex<BTree>>,
    bminus_tree: Arc<Mutex<BMinusTree>>,
}

#[derive(Deserialize)]
struct InsertReq {
    key: u64,
    value: String,
}

#[derive(Deserialize)]
struct DeleteReq {
    key: u64,
}

#[derive(Deserialize)]
struct SearchReq {
    key: u64,
}

#[derive(Deserialize)]
struct ResetReq {
    max_keys: usize,
}

#[tokio::main]
async fn main() {
    // initialize disk connection for B+ Tree
    let pager_plus = Arc::new(Mutex::new(Pager::new("bplus_tree.db").unwrap()));
    let btree = Arc::new(Mutex::new(BTree::new(pager_plus, 3)));
    
    // initialize disk connection for B- Tree
    let pager_minus = Arc::new(Mutex::new(Pager::new("bminus_tree.db").unwrap()));
    let bminus_tree = Arc::new(Mutex::new(BMinusTree::new(pager_minus, 3)));
    
    let state = AppState { btree, bminus_tree };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        .route("/tree", get(get_tree_plus))
        .route("/insert", post(insert_key_plus))
        .route("/delete", post(delete_key_plus))
        .route("/search", post(search_key_plus))
        .route("/btree", get(get_tree_minus))
        .route("/insert_btree", post(insert_key_minus))
        .route("/delete_btree", post(delete_key_minus))
        .route("/search_btree", post(search_key_minus))
        .route("/reset", post(reset_trees))
        .layer(cors)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("Server listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}

// B+ TREE ENDPOINTS
async fn get_tree_plus(State(state): State<AppState>) -> Json<serde_json::Value> {
    let tree = state.btree.lock().unwrap();
    Json(tree.get_tree_json())
}

async fn insert_key_plus(
    State(state): State<AppState>,
    Json(payload): Json<InsertReq>,
) -> Json<serde_json::Value> {
    let mut tree = state.btree.lock().unwrap();
    tree.insert(payload.key, payload.value);
    Json(tree.get_tree_json())
}

async fn delete_key_plus(
    State(state): State<AppState>,
    Json(payload): Json<DeleteReq>,
) -> Json<serde_json::Value> {
    let mut tree = state.btree.lock().unwrap();
    tree.delete(payload.key);
    Json(tree.get_tree_json())
}

// B- TREE ENDPOINTS
async fn get_tree_minus(State(state): State<AppState>) -> Json<serde_json::Value> {
    let tree = state.bminus_tree.lock().unwrap();
    Json(tree.get_tree_json())
}

async fn insert_key_minus(
    State(state): State<AppState>,
    Json(payload): Json<InsertReq>,
) -> Json<serde_json::Value> {
    let mut tree = state.bminus_tree.lock().unwrap();
    tree.insert(payload.key, payload.value);
    Json(tree.get_tree_json())
}

async fn delete_key_minus(
    State(state): State<AppState>,
    Json(payload): Json<DeleteReq>,
) -> Json<serde_json::Value> {
    let mut tree = state.bminus_tree.lock().unwrap();
    tree.delete(payload.key);
    Json(tree.get_tree_json())
}

async fn search_key_plus(
    State(state): State<AppState>,
    Json(payload): Json<SearchReq>,
) -> Json<serde_json::Value> {
    let tree = state.btree.lock().unwrap();
    let path = tree.search_path(payload.key);
    Json(serde_json::json!({
        "path": path,
        "key": payload.key,
        "found": path.last().map(|node| node.get("found").and_then(|v| v.as_bool()).unwrap_or(false)).unwrap_or(false)
    }))
}

async fn search_key_minus(
    State(state): State<AppState>,
    Json(payload): Json<SearchReq>,
) -> Json<serde_json::Value> {
    let tree = state.bminus_tree.lock().unwrap();
    let path = tree.search_path(payload.key);
    Json(serde_json::json!({
        "path": path,
        "key": payload.key,
        "found": path.last().map(|node| node.get("found").and_then(|v| v.as_bool()).unwrap_or(false)).unwrap_or(false)
    }))
}

async fn reset_trees(
    State(state): State<AppState>,
    Json(payload): Json<ResetReq>,
) -> Json<serde_json::Value> {
    let mut btree = state.btree.lock().unwrap();
    btree.reset(payload.max_keys);
    
    let mut bminus_tree = state.bminus_tree.lock().unwrap();
    bminus_tree.reset(payload.max_keys);
    
    Json(serde_json::json!({ "status": "ok" }))
}
