use super::*;

#[instrument(skip(socket, state))]
pub async fn ws_handler(mut socket: WebSocket, user: Uuid, state: AppState) {}
