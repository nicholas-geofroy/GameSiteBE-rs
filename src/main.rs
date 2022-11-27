mod games;
mod lobby;
mod lobby_manager;
mod models;
mod socket;
mod user_manager;
use axum::{extract::Extension, http::Method};
use lobby_manager::LobbyManager;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

use axum::{
    extract::{ws::WebSocket, Path, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::get,
    Router, TypedHeader,
};

#[tokio::main]
async fn main() {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin(Any);

    let lm = Arc::new(Mutex::new(LobbyManager::new()));
    let app = Router::new()
        .route("/", get(handler))
        .route("/lobby/:id/ws", get(ws_handler))
        .layer(Extension(lm))
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 9000));
    println!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

async fn ws_handler<'a>(
    Path(lobby_id): Path<String>,
    Extension(lm): Extension<Arc<Mutex<LobbyManager>>>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
) -> impl IntoResponse {
    if let Some(TypedHeader(user_agent)) = user_agent {
        println!(
            "`{}` trying to connect to lobby {}",
            user_agent.as_str(),
            lobby_id
        );
    }

    ws.on_upgrade(move |socket: WebSocket| socket::handle_socket(socket, lm, lobby_id))
}
