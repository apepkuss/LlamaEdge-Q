use crate::error::{ServerError, ServerResult};
use async_trait::async_trait;
use axum::http::Uri;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;
use tracing::warn;

pub(crate) type ServerId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ServerIdToRemove {
    pub id: ServerId,
}

/// Represents a LlamaEdge API server
#[derive(Debug, Serialize)]
pub struct Server {
    pub id: ServerId,
    pub url: String,
    pub kind: ServerKind,
    #[serde(skip)]
    connections: AtomicUsize,
}
impl<'de> Deserialize<'de> for Server {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Create a helper struct to deserialize into
        #[derive(Deserialize)]
        struct ServerHelper {
            url: String,
            kind: ServerKind,
        }

        // Deserialize into the helper struct
        let helper = ServerHelper::deserialize(deserializer)?;

        let kind = helper.kind.to_string().trim().replace(',', "-");
        let id = format!("{}-server-{}", kind, uuid::Uuid::new_v4());

        // Create the actual Server instance
        Ok(Server {
            id,
            url: helper.url,
            kind: helper.kind,
            connections: AtomicUsize::new(0),
        })
    }
}
impl Clone for Server {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            url: self.url.clone(),
            kind: self.kind,
            connections: AtomicUsize::new(self.connections.load(Ordering::Relaxed)),
        }
    }
}

#[test]
fn test_deserialize_server() {
    let serialized = r#"{"url": "http://localhost:8000", "kind": "chat,tts"}"#;
    let server: Server = serde_json::from_str(serialized).unwrap();
    println!("id: {}", server.id);
    assert_eq!(server.url, "http://localhost:8000");
    assert_eq!(server.kind, ServerKind::chat | ServerKind::tts);

    let serialized = r#"{"url": "http://localhost:8000", "kind": "chat"}"#;
    let server: Server = serde_json::from_str(serialized).unwrap();
    println!("id: {}", server.id);
    assert_eq!(server.url, "http://localhost:8000");
    assert_eq!(server.kind, ServerKind::chat);
}

#[test]
fn test_serialize_server() {
    let id = "chat-tts-29b6c973-d45a-4487-a3da-2e9b1f704fd9".to_string();
    let server = Server {
        id,
        url: "http://localhost:8000".to_string(),
        kind: ServerKind::chat | ServerKind::tts,
        connections: AtomicUsize::new(0),
    };
    let serialized = serde_json::to_string(&server).unwrap();
    assert_eq!(
        serialized,
        r#"{"id":"chat-tts-29b6c973-d45a-4487-a3da-2e9b1f704fd9","url":"http://localhost:8000","kind":"chat,tts"}"#
    );

    let id = "chat-2424f42e-fcfb-458e-9a6a-ad419e24b5f5".to_string();
    let server: Server = Server {
        id,
        url: "http://localhost:8000".to_string(),
        kind: ServerKind::chat,
        connections: AtomicUsize::new(0),
    };
    let serialized = serde_json::to_string(&server).unwrap();
    assert_eq!(
        serialized,
        r#"{"id":"chat-2424f42e-fcfb-458e-9a6a-ad419e24b5f5","url":"http://localhost:8000","kind":"chat"}"#
    );
}

bitflags! {
    /// Represents the kind of server
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ServerKind: u32{
        const chat = 0b00000001;
        const embeddings = 0b00000010;
        const image = 0b00000100;
        const tts = 0b00001000;
        const translate = 0b00010000;
        const transcribe = 0b00100000;
    }
}
impl std::fmt::Display for ServerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut kind_str = String::new();
        if self.contains(ServerKind::chat) {
            kind_str.push_str("chat,");
        }
        if self.contains(ServerKind::embeddings) {
            kind_str.push_str("embeddings,");
        }
        if self.contains(ServerKind::image) {
            kind_str.push_str("image,");
        }
        if self.contains(ServerKind::tts) {
            kind_str.push_str("tts,");
        }
        if self.contains(ServerKind::translate) {
            kind_str.push_str("translate,");
        }
        if self.contains(ServerKind::transcribe) {
            kind_str.push_str("transcribe,");
        }

        if !kind_str.is_empty() {
            kind_str = kind_str.trim_end_matches(',').to_string();
        }

        write!(f, "{}", kind_str)
    }
}
impl std::str::FromStr for ServerKind {
    type Err = ServerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ss = s.to_lowercase();
        let values = ss.split(',').collect::<Vec<&str>>();
        let mut kind = Self::empty();
        for val in values {
            match val.trim() {
                "chat" => kind.set(Self::chat, true),
                "embeddings" => kind.set(Self::embeddings, true),
                "image" => kind.set(Self::image, true),
                "tts" => kind.set(Self::tts, true),
                "translate" => kind.set(Self::translate, true),
                "transcribe" => kind.set(Self::transcribe, true),
                _ => return Err(ServerError::InvalidServerKind(s.to_string())),
            }
        }
        Ok(kind)
    }
}
impl Serialize for ServerKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Convert the flags to a string representation
        let mut kind_str = String::new();
        if self.contains(ServerKind::chat) {
            kind_str.push_str("chat,");
        }
        if self.contains(ServerKind::embeddings) {
            kind_str.push_str("embeddings,");
        }
        if self.contains(ServerKind::image) {
            kind_str.push_str("image,");
        }
        if self.contains(ServerKind::tts) {
            kind_str.push_str("tts,");
        }
        if self.contains(ServerKind::translate) {
            kind_str.push_str("translate,");
        }
        if self.contains(ServerKind::transcribe) {
            kind_str.push_str("transcribe,");
        }

        // Remove trailing comma if present
        if !kind_str.is_empty() {
            kind_str.pop();
        }

        // Serialize as a string
        serializer.serialize_str(&kind_str)
    }
}
impl<'de> Deserialize<'de> for ServerKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // First deserialize into a String
        let s = String::deserialize(deserializer)?;

        // Parse the string using from_str
        s.parse::<ServerKind>()
            .map_err(|e| serde::de::Error::custom(format!("Failed to parse ServerKindNew: {}", e)))
    }
}
impl std::hash::Hash for ServerKind {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.bits().hash(state);
    }
}

#[test]
fn test_serialize_server_kind() {
    let kind = ServerKind::chat | ServerKind::tts;
    let serialized = serde_json::to_string(&kind).unwrap();
    assert_eq!(serialized, "\"chat,tts\"");

    let kind = ServerKind::chat;
    let serialized = serde_json::to_string(&kind).unwrap();
    assert_eq!(serialized, "\"chat\"");
}

#[test]
fn test_deserialize_server_kind() {
    let serialized = "\"chat,tts\"";
    let kind: ServerKind = serde_json::from_str(serialized).unwrap();
    assert_eq!(kind, ServerKind::chat | ServerKind::tts);

    let serialized = "\"chat\"";
    let kind: ServerKind = serde_json::from_str(serialized).unwrap();
    assert_eq!(kind, ServerKind::chat);
}

#[derive(Debug)]
pub(crate) struct ServerGroup {
    pub(crate) servers: RwLock<Vec<Server>>,
    pub(crate) ty: ServerKind,
}
impl ServerGroup {
    pub(crate) fn new(ty: ServerKind) -> Self {
        Self {
            servers: RwLock::new(Vec::new()),
            ty,
        }
    }

    pub(crate) async fn register(&mut self, server: &Server) -> ServerResult<()> {
        // check if the server is already registered
        if self
            .servers
            .read()
            .await
            .iter()
            .any(|s| s.url == server.url)
        {
            let err_msg = format!("Server already registered: {}", server.url);

            warn!(target: "stdout", "{}", &err_msg);

            return Err(ServerError::Operation(err_msg));
        }

        self.servers.write().await.push(server.clone());

        Ok(())
    }

    pub(crate) async fn unregister(&mut self, server_id: impl AsRef<str>) -> ServerResult<()> {
        self.servers
            .write()
            .await
            .retain(|s| s.id != server_id.as_ref());

        Ok(())
    }

    pub(crate) async fn is_empty(&self) -> bool {
        self.servers.read().await.is_empty()
    }
}
#[async_trait]
impl RoutingPolicy for ServerGroup {
    async fn next(&self) -> Result<Uri, ServerError> {
        if self.servers.read().await.is_empty() {
            return Err(ServerError::NotFoundServer(self.ty.to_string()));
        }

        let servers = self.servers.read().await;

        let server = if servers.len() == 1 {
            servers.first().unwrap()
        } else {
            servers
                .iter()
                .min_by(|s1, s2| {
                    s1.connections
                        .load(Ordering::Relaxed)
                        .cmp(&s2.connections.load(Ordering::Relaxed))
                })
                .unwrap()
        };

        server.connections.fetch_add(1, Ordering::Relaxed);
        Ok(server.url.parse().unwrap())
    }
}

#[async_trait]
pub(crate) trait RoutingPolicy: Sync + Send {
    async fn next(&self) -> Result<Uri, ServerError>;
}
