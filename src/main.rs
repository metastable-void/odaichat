
mod model;

use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    http::StatusCode, response::IntoResponse, routing::{any, get, post}, Json, Router,
    body::Bytes,
};
use model::Command;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
enum ApiCommand {
    #[serde(rename = "snake_case")]
    SetCanvas {
        canvas_id: String,
    },
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(model): State<model::OdaiChat>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, model))
}

async fn handle_socket(mut socket: WebSocket, model: model::OdaiChat) {
    let mut subscribed_canvas_id = None;
    let mut rx = model.get_command_receiver();
    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                let msg = if let Ok(msg) = msg {
                    msg
                } else {
                    // client disconnected
                    return;
                };

                match msg {
                    Message::Text(json) => {
                        let cmd = match serde_json::from_str::<ApiCommand>(json.as_str()) {
                            Err(_e) => continue,
                            Ok(cmd) => cmd,
                        };

                        match cmd {
                            ApiCommand::SetCanvas { canvas_id } => {
                                subscribed_canvas_id = Some(canvas_id);
                            },
                        }
                    },

                    Message::Binary(bytes) => {
                        let canvas_id = if let Some(id) = subscribed_canvas_id.as_ref() {
                            id.to_owned().leak().into()
                        } else {
                            continue;
                        };

                        model.send_command(Command::UpdateCanvas { canvas_id, png_bytes: bytes.to_vec().into() })
                    },

                    _ => continue,
                }
            },
            Ok(cmd) = rx.recv() => {
                let my_canvas_id = if let Some(id) = subscribed_canvas_id.as_ref() {
                    id
                } else {
                    continue;
                };
                match cmd {
                    Command::UpdateCanvas { canvas_id, png_bytes } => {
                        if canvas_id.as_ref() != my_canvas_id {
                            continue;
                        }

                        if let Err(_e) = socket.send(Message::Binary(Bytes::from(png_bytes.as_ref().to_owned()))).await {
                            break;
                        }
                    }
                }
            },
            else => break,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenvy::dotenv().ok();
    let db_path = std::env::var("DB_PATH").unwrap_or("/tmp/odaichat.db".to_string());
    let listen_addr = std::env::var("LISTEN_ADDR").unwrap_or("[::]:3333".to_string());
    let model = model::OdaiChat::open(&db_path)?;
    let app = Router::new()
        .route("/ws", any(ws_handler))
        .with_state::<()>(model);

    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
    println!("Listening on: {}", &listen_addr);
    axum::serve(listener, app).await?;
    
    Ok(())
}
