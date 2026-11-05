use crate::network::{Command, Event, PROTOCOL};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use std::{
    net::{SocketAddr, TcpListener},
    sync::Arc,
    thread,
    time::Duration,
};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    sync::{oneshot, Semaphore},
};
use tokio_util::codec::{FramedRead, LinesCodec};
include!(concat!(env!("OUT_DIR"), "/web_assets.rs"));
#[derive(Clone)]
struct HostState {
    target: SocketAddr,
    slots: Arc<Semaphore>,
}
pub struct WebHost {
    pub address: SocketAddr,
    stop: Option<oneshot::Sender<()>>,
    handle: Option<thread::JoinHandle<()>>,
}
impl WebHost {
    pub fn available() -> bool {
        ASSETS.iter().any(|(p, _, _)| *p == "/commons_bg.wasm")
    }
    pub fn start(bind: &str, target: SocketAddr) -> Result<Self, String> {
        if !Self::available() {
            return Err("The browser package has not been built. Run the included web packaging tool first.".into());
        }
        let listener = TcpListener::bind(bind).map_err(|e| {
            format!("Could not open browser port {bind}: {e}. Choose another browser port.")
        })?;
        listener.set_nonblocking(true).map_err(|e| e.to_string())?;
        let address = listener.local_addr().map_err(|e| e.to_string())?;
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let (stop, stopped) = oneshot::channel();
        let handle = thread::spawn(move || {
            runtime.block_on(async move{
   let state=HostState{target,slots:Arc::new(Semaphore::new(64))};
   let app=Router::new().route("/ws",get(upgrade)).route("/api/info",get(||async{axum::Json(serde_json::json!({"protocol":PROTOCOL,"client_only":true,"app":"Commons","media":"pcm16k-jpeg"}))})).fallback(get(asset)).with_state(state);
   let listener=tokio::net::TcpListener::from_std(listener).expect("nonblocking listener");
   tokio::select!{result=axum::serve(listener,app)=>{if let Err(e)=result{eprintln!("Browser server stopped: {e}");}},_=stopped=>{}}
  });
        });
        Ok(Self {
            address,
            stop: Some(stop),
            handle: Some(handle),
        })
    }
}
impl Drop for WebHost {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
async fn asset(uri: axum::http::Uri, headers: HeaderMap) -> Response {
    let path = if uri.path() == "/" {
        "/index.html"
    } else {
        uri.path()
    };
    let Some((_, plain, gzip)) = ASSETS.iter().find(|(p, _, _)| *p == path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let compress = headers
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .any(|s| s.trim().split(';').next() == Some("gzip"))
        })
        && !gzip.is_empty();
    let mime = if path.ends_with(".wasm") {
        "application/wasm"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".png") {
        "image/png"
    } else {
        "text/plain; charset=utf-8"
    };
    let mut response = (
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "no-cache"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::REFERRER_POLICY, "same-origin"),
            (header::VARY, "Accept-Encoding"),
        ],
        if compress { *gzip } else { *plain },
    )
        .into_response();
    if compress {
        response.headers_mut().insert(
            header::CONTENT_ENCODING,
            axum::http::HeaderValue::from_static("gzip"),
        );
    }
    response
}
fn origin_allowed(headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN) else {
        return true;
    };
    let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    let Ok(origin) = origin.to_str().unwrap_or("").parse::<url::Url>() else {
        return false;
    };
    matches!(origin.scheme(), "http" | "https")
        && origin[url::Position::BeforeHost..url::Position::AfterPort].eq_ignore_ascii_case(host)
}
async fn upgrade(
    State(state): State<HostState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if !origin_allowed(&headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Ok(slot) = state.slots.clone().try_acquire_owned() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    ws.max_message_size(1048576)
        .max_frame_size(1048576)
        .on_upgrade(move |socket| async move {
            let _slot = slot;
            bridge(socket, state.target).await;
        })
}
async fn bridge(mut socket: WebSocket, target: SocketAddr) {
    let Ok(Ok(stream)) = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect(target),
    )
    .await
    else {
        let _ = socket
            .send(Message::Text(
                serde_json::to_string(&Event::Error("The workspace server is offline.".into()))
                    .unwrap()
                    .into(),
            ))
            .await;
        return;
    };
    let _ = stream.set_nodelay(true);
    let (read, mut write) = stream.into_split();
    let mut lines = FramedRead::new(
        BufReader::new(read),
        LinesCodec::new_with_max_length(crate::network::MAX_SERVER_EVENT_BYTES),
    );
    loop {
        tokio::select! {
         message=socket.recv()=>{match message{
          Some(Ok(Message::Text(text)))=>{let Ok(mut command)=serde_json::from_str::<Command>(&text)else{break;};
           if let Command::Hello{editor_key,..} = &mut command { editor_key.clear(); }
           let mut bytes=serde_json::to_vec(&command).unwrap();bytes.push(b'\n');if !matches!(tokio::time::timeout(Duration::from_secs(2),write.write_all(&bytes)).await,Ok(Ok(()))){break;}
          },Some(Ok(Message::Ping(data)))=>{if socket.send(Message::Pong(data)).await.is_err(){break;}},Some(Ok(Message::Pong(_)))=>{},_=>break,
         }}
         line=lines.next()=>{let Some(Ok(text))=line else{break;};if !matches!(tokio::time::timeout(Duration::from_secs(2),socket.send(Message::Text(text.into()))).await,Ok(Ok(()))){break;}}
        }
    }
    let _ = socket.close().await;
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn browser_origin_must_match_the_host() {
        let mut h = HeaderMap::new();
        h.insert(header::HOST, "office.example:8080".parse().unwrap());
        h.insert(
            header::ORIGIN,
            "http://office.example:8080".parse().unwrap(),
        );
        assert!(origin_allowed(&h));
        h.insert(header::ORIGIN, "https://other.example".parse().unwrap());
        assert!(!origin_allowed(&h));
        h.insert(header::ORIGIN, "null".parse().unwrap());
        assert!(!origin_allowed(&h));
    }
}
#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{
        avatars::Avatar,
        network::{Client, Server},
        world,
    };
    use std::{
        io::{Read, Write},
        net::TcpStream,
        time::Instant,
    };
    fn event(client: &Client, predicate: impl Fn(&Event) -> bool) -> Event {
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            let received = client
                .rx
                .recv_timeout(end.saturating_duration_since(Instant::now()))
                .expect("server event timeout");
            if predicate(&received) {
                return received;
            }
            assert!(!matches!(received, Event::Error(_)), "{received:?}");
        }
    }
    fn request(port: u16, path: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(
            stream,
            "HEAD {path} HTTP/1.1\r\nHost: localhost:{port}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response.to_lowercase()
    }
    #[test]
    fn static_package_has_correct_types_and_exposes_only_bundled_files() {
        let mut server = Server::start_with_rooms(
            "127.0.0.1:0",
            "Files".into(),
            "".into(),
            world::default_world(),
        )
        .unwrap();
        server.enable_web("127.0.0.1:0").unwrap();
        let port = server.web_port().unwrap();
        for (path, mime) in [
            ("/", "text/html"),
            ("/style.css", "text/css"),
            ("/commons.js", "text/javascript"),
            ("/commons_bg.wasm", "application/wasm"),
            ("/audio-worklet.js", "text/javascript"),
        ] {
            let response = request(port, path);
            assert!(response.starts_with("http/1.1 200"), "{path}: {response}");
            assert!(
                response.contains(&format!("content-type: {mime}")),
                "{path}: {response}"
            );
        }
        for path in ["/data/settings.json", "/../data/world.json", "/Cargo.toml"] {
            assert!(request(port, path).starts_with("http/1.1 404"));
        }
    }
    #[test]
    fn browser_transport_shares_desktop_state_and_strips_editor_privileges() {
        let mut server = Server::start_with_rooms(
            "127.0.0.1:0",
            "Shared".into(),
            "secret".into(),
            world::default_world(),
        )
        .unwrap();
        server.enable_web("127.0.0.1:0").unwrap();
        let owner = Client::connect_custom(
            server.address.to_string(),
            "Desktop host".into(),
            "secret".into(),
            0,
            Avatar::default(),
            server.editor_key.clone(),
        );
        event(&owner, |e| matches!(e, Event::Welcome(_)));
        let guest = Client::connect_custom(
            format!("http://127.0.0.1:{}", server.web_port().unwrap()),
            "Browser guest".into(),
            "secret".into(),
            0,
            Avatar {
                body: 1,
                hair: 9,
                ..Default::default()
            },
            server.editor_key.clone(),
        );
        event(&guest, |e| matches!(e, Event::Welcome(_)));
        event(
            &owner,
            |e| matches!(e,Event::State(s)if s.people.len()==2&&s.people.iter().any(|p|p.avatar.body==1)),
        );
        let mut design = world::preset(2, "Host design", 32, 24);
        design.id = 1;
        guest
            .tx
            .send(Command::EditRoom {
                room: 0,
                map: design.clone(),
            })
            .unwrap();
        event(
            &guest,
            |e| matches!(e,Event::Notice(s)if s.contains("cannot edit")),
        );
        let before = event(&guest, |e| matches!(e, Event::State(_)));
        if let Event::State(s) = before {
            assert_ne!(s.rooms[0], design);
        }
        owner
            .tx
            .send(Command::EditRoom {
                room: 0,
                map: design.clone(),
            })
            .unwrap();
        event(&guest, |e| matches!(e,Event::State(s)if s.rooms[0]==design));
        guest
            .tx
            .send(Command::Chat {
                text: "Hello from a browser".into(),
                room: None,
                scope: None,
            })
            .unwrap();
        event(
            &owner,
            |e| matches!(e,Event::State(s)if s.messages.iter().any(|m|m.text=="Hello from a browser")),
        );
        server.web.take();
        event(&guest, |e| {
            matches!(e, Event::Error(_) | Event::Disconnected)
        });
    }
}

#[cfg(test)]
mod failure_tests {
    use super::*;
    use crate::{
        avatars::Avatar,
        network::{Client, Server},
        world,
    };
    use tungstenite::client::IntoClientRequest;
    #[test]
    fn websocket_rejects_cross_site_origins_and_wrong_passwords() {
        let mut server = Server::start_with_rooms(
            "127.0.0.1:0",
            "Private".into(),
            "correct".into(),
            world::default_world(),
        )
        .unwrap();
        server.enable_web("127.0.0.1:0").unwrap();
        let url = format!("http://127.0.0.1:{}", server.web_port().unwrap());
        let mut request = crate::network::websocket_url(&url)
            .unwrap()
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert("Origin", "https://other.example".parse().unwrap());
        let response = tungstenite::connect(request);
        assert!(matches!(response,Err(tungstenite::Error::Http(r))if r.status().as_u16()==403));
        let client = Client::connect_custom(
            url,
            "Guest".into(),
            "wrong".into(),
            0,
            Avatar::default(),
            String::new(),
        );
        let first = client.rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(matches!(first,Event::Error(s)if s.contains("password")));
    }
}
#[cfg(test)]
mod performance_tests {
    use super::*;
    use crate::{network::Server, world};
    use tungstenite::{stream::MaybeTlsStream, Message as WsMessage};
    #[test]
    fn walking_updates_omit_the_unchanged_world_after_the_initial_transfer() {
        let mut server = Server::start_with_rooms(
            "127.0.0.1:0",
            "Lean office".into(),
            "".into(),
            world::default_world(),
        )
        .unwrap();
        server.enable_web("127.0.0.1:0").unwrap();
        let (mut socket, _) =
            tungstenite::connect(format!("ws://127.0.0.1:{}/ws", server.web_port().unwrap()))
                .unwrap();
        if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
            stream
                .set_read_timeout(Some(Duration::from_secs(4)))
                .unwrap();
        }
        socket
            .send(WsMessage::Text(
                serde_json::to_string(&Command::Hello {
                    name: "Web guest".into(),
                    password: "".into(),
                    color: 0,
                    avatar: Default::default(),
                    editor_key: String::new(),
                    protocol: PROTOCOL,
                })
                .unwrap()
                .into(),
            ))
            .unwrap();
        let full = loop {
            if let WsMessage::Text(text) = socket.read().unwrap() {
                if let Event::State(state) = serde_json::from_str::<Event>(&text).unwrap() {
                    assert_eq!(state.rooms.len(), 4);
                    break text.len();
                }
            }
        };
        socket
            .send(WsMessage::Text(
                serde_json::to_string(&Command::Motion {
                    room: 0,
                    x: 0.55,
                    y: 0.65,
                    direction: 3,
                    walking: true,
                })
                .unwrap()
                .into(),
            ))
            .unwrap();
        loop {
            if let WsMessage::Text(text) = socket.read().unwrap() {
                if let Event::State(state) = serde_json::from_str::<Event>(&text).unwrap() {
                    assert!(state.rooms.is_empty());
                    assert!(
                        text.len() * 10 < full,
                        "Walking packet should be much smaller than a full office"
                    );
                    break;
                }
            }
        }
    }
}
