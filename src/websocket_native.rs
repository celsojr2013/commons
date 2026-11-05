use super::*;
use tungstenite::{protocol::WebSocketConfig, stream::MaybeTlsStream, Message};
pub fn run(
    address: String,
    hello: Command,
    commands: mpsc::Receiver<Command>,
    events: mpsc::Sender<Event>,
    stop: Arc<AtomicBool>,
) -> Result<(), String> {
    let endpoint = crate::network::websocket_url(&address)?;
    let url = url::Url::parse(&endpoint).map_err(|e| e.to_string())?;
    let host = url.host_str().ok_or("Server name missing")?;
    let port = url.port_or_known_default().ok_or("Server port missing")?;
    let mut stream = None;
    for addr in (host, port).to_socket_addrs().map_err(|e| e.to_string())? {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        if let Ok(s) = TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
            stream = Some(s);
            break;
        }
    }
    let stream =
        stream.ok_or("Could not reach the browser server. Check the host and HTTPS address.")?;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_nodelay(true);
    let mut config = WebSocketConfig::default();
    config.max_message_size = Some(MAX_SERVER_EVENT_BYTES);
    config.max_frame_size = Some(MAX_SERVER_EVENT_BYTES);
    config.write_buffer_size = 0;
    config.max_write_buffer_size = 1_048_576;
    let (mut socket, _) = tungstenite::client_tls_with_config(endpoint, stream, Some(config), None)
        .map_err(|e| format!("Could not open this browser invite: {e}"))?;
    match socket.get_mut() {
        MaybeTlsStream::Plain(s) => s.set_nonblocking(true),
        MaybeTlsStream::Rustls(s) => s.sock.set_nonblocking(true),
        _ => return Err("Unsupported secure connection.".into()),
    }
    .map_err(|e| e.to_string())?;
    let mut pending = VecDeque::from([hello]);
    let mut world_cache = crate::network::world_sync::WorldCache::default();
    while !stop.load(Ordering::Relaxed) {
        for cmd in commands.try_iter() {
            if pending.len() < 128 || !matches!(cmd, Command::Media(_)) {
                pending.push_back(cmd);
            }
        }
        while let Some(cmd) = pending.pop_front() {
            match socket.send(Message::Text(serde_json::to_string(&cmd).unwrap().into())) {
                Ok(()) => {}
                Err(tungstenite::Error::WriteBufferFull(_)) => {
                    if !matches!(cmd, Command::Media(_)) {
                        pending.push_front(cmd);
                    }
                    break;
                }
                Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break
                }
                Err(e) => return Err(format!("The browser server connection closed: {e}")),
            }
        }
        match socket.flush() {
            Ok(()) => {}
            Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => return Err("The browser server connection closed.".into()),
        }
        loop {
            match socket.read() {
                Ok(Message::Text(text)) => {
                    let mut event: Event = serde_json::from_str(&text)
                        .map_err(|_| "The browser server sent an incompatible response.")?;
                    if matches!(&event,Event::State(s)if s.protocol!=PROTOCOL) {
                        return Err("This server requires the current Commons version.".into());
                    }
                    if let Event::State(ref mut state) = event {
                        world_cache.hydrate(state);
                    }
                    let failed = matches!(event, Event::Error(_));
                    let _ = events.send(event);
                    if failed {
                        return Ok(());
                    }
                }
                Ok(Message::Close(_)) => {
                    return Err(
                        "The host closed this workspace. Reconnect when it is running again."
                            .into(),
                    )
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    break
                }
                Err(_) => return Err("The browser server went offline.".into()),
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
    let _ = socket.close(None);
    Ok(())
}
