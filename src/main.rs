mod games;
mod lobby;
mod models;
mod socket;
mod user_manager;
use axum::extract::Extension;
use lobby::LobbyManager;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

use axum::{
    extract::{ws::WebSocket, Path, WebSocketUpgrade},
    response::{Html, IntoResponse},
    routing::get,
    Router, TypedHeader,
};

#[tokio::main]
async fn main() {
    let lm = Arc::new(Mutex::new(LobbyManager::new()));
    let app = Router::new()
        .route("/", get(handler))
        .route("/lobby/:id/ws", get(ws_handler))
        .layer(Extension(lm));

    let addr = SocketAddr::from(([127, 0, 0, 1], 9000));
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
    println!("Try connecting to lobby {}", lobby_id);

    let lobby = {
        let mut lm = lm.lock().await;

        let lobby = lm.get(lobby_id).clone();
        lobby
    };

    if let Some(TypedHeader(user_agent)) = user_agent {
        println!("`{}` connected to lobby {}", user_agent.as_str(), lobby.id);
    }

    ws.on_upgrade(move |socket: WebSocket| socket::handle_socket(socket, lobby))
}
