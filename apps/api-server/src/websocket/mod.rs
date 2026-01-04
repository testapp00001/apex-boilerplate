//! WebSocket handlers using socketioxide.

use socketioxide::{
    SocketIo,
    extract::{Data, SocketRef},
};
use std::sync::Arc;

use apex_infra::InMemoryPubSub;

/// Shared state for WebSocket handlers.
#[derive(Clone)]
pub struct WsState {
    pub pubsub: Arc<InMemoryPubSub>,
}

/// Configure WebSocket handlers.
pub fn configure_socket_handlers(io: SocketIo, _state: WsState) {
    io.ns("/", move |socket: SocketRef| {
        async move {
            let socket_id = socket.id.to_string();
            tracing::info!(socket_id = %socket_id, "Client connected");

            // Handle join room
            socket.on("join", |socket: SocketRef, Data::<String>(room)| async move {
                socket.join(room.clone()).ok();
                tracing::info!(socket_id = %socket.id, room = %room, "Client joined room");
                socket.emit("joined", &room).ok();
            });

            // Handle leave room
            socket.on("leave", |socket: SocketRef, Data::<String>(room)| async move {
                socket.leave(room.clone()).ok();
                tracing::info!(socket_id = %socket.id, room = %room, "Client left room");
            });

            // Handle broadcast to room
            socket.on("broadcast", |socket: SocketRef, Data::<(String, serde_json::Value)>(data)| async move {
                let (room, message) = data;
                tracing::debug!(socket_id = %socket.id, room = %room, "Broadcasting to room");
                socket.to(room).emit("message", &message).ok();
            });

            // Handle private message
            socket.on("private", |socket: SocketRef, Data::<(String, serde_json::Value)>(data)| async move {
                let (target_id, message) = data;
                tracing::debug!(socket_id = %socket.id, target = %target_id, "Sending private message");
                socket.to(target_id).emit("private_message", &message).ok();
            });

            // Handle ping
            socket.on("ping", |socket: SocketRef| async move {
                socket.emit("pong", &chrono::Utc::now().to_rfc3339()).ok();
            });

            // Handle disconnect
            socket.on_disconnect(|socket: SocketRef| async move {
                tracing::info!(socket_id = %socket.id, "Client disconnected");
            });
        }
    });
}

/// Create SocketIO layer for integration.
pub fn create_socketio_layer(state: WsState) -> (socketioxide::layer::SocketIoLayer, SocketIo) {
    let (layer, io) = SocketIo::new_layer();
    configure_socket_handlers(io.clone(), state);
    (layer, io)
}
