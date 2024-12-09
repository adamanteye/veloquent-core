use super::*;

use futures::{sink::SinkExt, stream::StreamExt};
use std::time::Duration;
use tokio::time::timeout;

#[doc(hidden)]
#[derive(Clone, Debug, Default)]
pub struct WebSocketPool {
    senders: Arc<DashMap<Uuid, Arc<Mutex<SplitSink<WebSocket, WebSocketMessage>>>>>,
    receivers: Arc<DashMap<Uuid, Arc<Mutex<SplitStream<WebSocket>>>>>,
}

impl WebSocketPool {
    #[instrument(skip(self, ws))]
    pub async fn register(&mut self, user: Uuid, ws: WebSocket) {
        event!(Level::INFO, "registered websocket for user [{}]", user);
        let (sender, receiver) = ws.split();
        self.senders.insert(user, Arc::new(Mutex::new(sender)));
        self.receivers.insert(user, Arc::new(Mutex::new(receiver)));
        self.listen(user).await;
    }

    #[instrument(skip(self))]
    pub async fn notify(&self, user: Uuid, message: Result<WebSocketMessage, AppError>) {
        if let Some(ws) = self.senders.get_mut(&user) {
            if let Ok(message) = message {
                event!(
                    Level::INFO,
                    "websocket sent message [{message:?}] to user [{user}]",
                );
                ws.lock().await.send(message).await.ok();
            }
        }
    }

    #[instrument(skip(self))]
    async fn listen(&self, user: Uuid) {
        if let Some(receiver) = self.receivers.get(&user) {
            let receiver = receiver.clone();
            event!(Level::INFO, "websocket listening for user [{user}]");
            tokio::spawn(async move {
                loop {
                    let mut receiver_lock = receiver.lock().await;
                    match receiver_lock.next().await {
                        Some(Ok(WebSocketMessage::Text(text))) => {
                            event!(
                                Level::DEBUG,
                                "websocket received message [{text}] from [{user}]",
                            );
                        }
                        _ => {}
                    }
                }
            });
        } else {
            event!(Level::WARN, "cannot find ws receiver for user [{}]", user);
        }
    }
}

#[instrument(skip(state, ws))]
pub async fn ws_upgrade_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    Ok(ws.on_upgrade(move |mut socket| async {
        let msg = timeout(Duration::from_millis(2000), socket.recv()).await;
        if let Ok(msg) = msg {
            if let Some(msg) = msg {
                if let Ok(msg) = msg {
                    match msg {
                        WebSocketMessage::Text(t) => {
                            let token: Result<JWTPayload, AppError> = t.as_str().try_into();
                            match token {
                                Ok(payload) => {
                                    let mut pool = state.ws_pool;
                                    pool.register(payload.id, socket).await;
                                }
                                Err(e) => {
                                    event!(Level::ERROR, "websocket received invalid jwt [{e:?}]",);
                                    return;
                                }
                            }
                        }
                        _ => {
                            return;
                        }
                    }
                } else {
                    return;
                }
            }
        } else {
            event!(Level::DEBUG, "websocket await jwt timeout");
        }
    }))
}
