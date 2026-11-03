use crate::{
    avatars::Avatar,
    world::{RoomMap, TalkScope},
};

use base64::{engine::general_purpose::STANDARD, Engine};

use serde::{Deserialize, Serialize};

pub const PROTOCOL: u16 = 4;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Person {
    pub id: u64,
    pub name: String,
    pub room: usize,
    pub x: f32,
    pub y: f32,
    pub color: usize,
    pub away: bool,
    #[serde(default)]
    pub avatar: Avatar,
    #[serde(default)]
    pub direction: u8,
    #[serde(default)]
    pub walking: bool,
    #[serde(default)]
    pub mic: bool,
    #[serde(default)]
    pub camera: bool,
}

impl Person {
    pub fn scope(&self, rooms: &[RoomMap]) -> TalkScope {
        rooms[self.room].scope(self.room, [self.x, self.y])
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Message {
    pub name: String,
    pub text: String,
    pub room: Option<usize>,
    pub time: u64,
    #[serde(default)]
    pub area: Option<u64>,
    #[serde(skip)]
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub recipients: Vec<u64>,
}

impl Message {
    #[cfg(not(target_arch = "wasm32"))]
    fn visible_to(&self, person: &Person, rooms: &[RoomMap]) -> bool {
        match self.room {
            None => true,
            Some(room) => {
                let scope = person.scope(rooms);

                scope.room == room
                    && scope.area == self.area
                    && (self.area.is_none() || self.recipients.contains(&person.id))
            }
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub space: String,
    pub people: Vec<Person>,
    pub messages: Vec<Message>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rooms: Vec<RoomMap>,
    #[serde(default)]
    pub world_revision: u64,
    #[serde(default)]
    pub protocol: u16,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum MediaKind {
    Audio,
    Video,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MediaPacket {
    pub kind: MediaKind,
    pub data: String,
    #[serde(default)]
    pub scope: Option<TalkScope>,
}

impl MediaPacket {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_bytes(kind: MediaKind, bytes: &[u8]) -> Self {
        Self {
            kind,
            data: STANDARD.encode(bytes),
            scope: None,
        }
    }

    pub fn with_scope(mut self, scope: TalkScope) -> Self {
        self.scope = Some(scope);
        self
    }
    pub fn bytes(&self) -> Result<Vec<u8>, String> {
        STANDARD.decode(&self.data).map_err(|e| e.to_string())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn valid(&self) -> bool {
        if self.data.len() > 131072 {
            return false;
        }

        self.bytes().is_ok_and(|b| match self.kind {
            MediaKind::Audio => b.len() == 640,
            MediaKind::Video => (4..=98304).contains(&b.len()),
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Command {
    Hello {
        name: String,
        password: String,
        color: usize,
        #[serde(default)]
        avatar: Avatar,
        #[serde(default)]
        editor_key: String,
        #[serde(default)]
        protocol: u16,
    },
    Move {
        room: usize,
        x: f32,
        y: f32,
    },
    Chat {
        text: String,
        room: Option<usize>,
        #[serde(default)]
        scope: Option<TalkScope>,
    },
    Away(bool),
    Style(Avatar),
    Motion {
        room: usize,
        x: f32,
        y: f32,
        direction: u8,
        walking: bool,
    },
    EditRoom {
        room: usize,
        map: RoomMap,
    },
    MediaStatus {
        mic: bool,
        camera: bool,
    },
    Media(MediaPacket),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Event {
    Welcome(u64),
    State(Snapshot),
    Error(String),
    Disconnected,
    Notice(String),
    ConversationChanged(TalkScope),
    Media {
        sender: u64,
        scope: TalkScope,
        packet: MediaPacket,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
}

impl Endpoint {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut value = text.trim();
        if ["http://", "https://", "ws://", "wss://"]
            .iter()
            .any(|p| value.starts_with(p))
        {
            let url = url::Url::parse(value).map_err(|_| "Use a valid server invite.")?;
            return Ok(Self {
                host: url
                    .host_str()
                    .ok_or("Server name missing")?
                    .trim_matches(['[', ']'])
                    .to_owned(),
                port: url.port_or_known_default().ok_or("Server port missing")?,
            });
        }

        for prefix in ["commons://", "http://", "https://"] {
            if let Some(v) = value.strip_prefix(prefix) {
                value = v;

                break;
            }
        }

        value = value.trim_end_matches('/');

        if value.is_empty() || value.chars().any(char::is_whitespace) || value.contains('/') {
            return Err("Use a server address such as localhost:7777 or 192.168.1.12:7777.".into());
        }

        let (host, port) = if let Some(v) = value.strip_prefix('[') {
            let (h, rest) = v.split_once(']').ok_or("Close the IPv6 address with ].")?;

            let p = if rest.is_empty() {
                7777
            } else {
                rest.strip_prefix(':')
                    .ok_or("Use [IPv6]:port.")?
                    .parse::<u16>()
                    .map_err(|_| "Use a port between 1 and 65535.")?
            };

            (h.to_owned(), p)
        } else if value.parse::<std::net::IpAddr>().is_ok() {
            (value.to_owned(), 7777)
        } else if let Some((h, p)) = value.rsplit_once(':') {
            (
                h.to_owned(),
                p.parse::<u16>()
                    .map_err(|_| "Use a port between 1 and 65535.")?,
            )
        } else {
            (value.to_owned(), 7777)
        };

        if host.is_empty() || port == 0 {
            return Err("Use a server name and a port between 1 and 65535.".into());
        }

        let host = if host == "0.0.0.0" {
            "127.0.0.1".into()
        } else {
            host
        };

        Ok(Self { host, port })
    }

    pub fn is_local(&self) -> bool {
        self.host.eq_ignore_ascii_case("localhost")
            || self
                .host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    }

    pub fn display(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

#[cfg_attr(target_arch = "wasm32", path = "network_web.rs")]
#[cfg_attr(not(target_arch = "wasm32"), path = "network_native.rs")]
mod transport;
pub use transport::*;
#[cfg(any(target_arch = "wasm32", test))]
#[path = "network_queue.rs"]
mod queue;
#[cfg(not(target_arch = "wasm32"))]
#[path = "world_sync.rs"]
mod world_sync;
pub fn websocket_url(address: &str) -> Result<String, String> {
    let address = address.trim();
    let normalized = if let Some(rest) = address.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = address.strip_prefix("http://") {
        format!("ws://{rest}")
    } else if address.starts_with("ws://") || address.starts_with("wss://") {
        address.to_owned()
    } else {
        return Err("Use the browser invite beginning with http:// or https://.".into());
    };
    let mut url = url::Url::parse(&normalized).map_err(|_| "Use a valid browser invite URL.")?;
    if url.host_str().is_none() || !url.username().is_empty() || url.password().is_some() {
        return Err("Use a browser invite without embedded credentials.".into());
    }
    url.set_path("/ws");
    url.set_query(None);
    url.set_fragment(None);
    Ok(url.into())
}
