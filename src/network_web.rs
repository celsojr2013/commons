use super::queue::EventQueue;
use super::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[derive(Clone, Default)]
pub struct Receiver(Rc<RefCell<EventQueue>>);
impl Receiver {
    pub fn try_iter(&self) -> std::collections::vec_deque::IntoIter<Event> {
        self.0.borrow_mut().drain()
    }
    fn send(&self, event: Event) -> Result<(), ()> {
        self.0.borrow_mut().push(event);
        Ok(())
    }
}
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Event as BrowserEvent, MessageEvent, WebSocket};
pub struct Sender(WebSocket);
impl Sender {
    pub fn send(&self, command: Command) -> Result<(), String> {
        if self.0.ready_state() != WebSocket::OPEN {
            return Err("The server connection is not open.".into());
        }
        if self.0.buffered_amount() > 262144 && matches!(command, Command::Media(_)) {
            return Ok(());
        }
        self.0
            .send_with_str(&serde_json::to_string(&command).map_err(|e| e.to_string())?)
            .map_err(|_| "Could not send to the server.".into())
    }
}
pub struct Client {
    pub tx: Sender,
    pub rx: Receiver,
    socket: WebSocket,
    _open: Closure<dyn FnMut(BrowserEvent)>,
    _message: Closure<dyn FnMut(MessageEvent)>,
    _error: Closure<dyn FnMut(BrowserEvent)>,
    _close: Closure<dyn FnMut(BrowserEvent)>,
}
impl Client {
    pub fn connect_custom(
        address: String,
        name: String,
        password: String,
        color: usize,
        avatar: Avatar,
        _editor_key: String,
    ) -> Self {
        let events = Receiver::default();
        let rx = events.clone();
        let had_error = Rc::new(Cell::new(false));
        let endpoint = websocket_url(&address);
        // A syntactically valid disconnected socket lets errors travel through the same UI channel.
        let socket = WebSocket::new(endpoint.as_deref().unwrap_or("ws://127.0.0.1:1/ws"))
            .expect("valid websocket URL");
        if let Err(error) = endpoint {
            let _ = events.send(Event::Error(error));
        }
        let hello = serde_json::to_string(&Command::Hello {
            name,
            password,
            color,
            avatar,
            editor_key: String::new(),
            protocol: PROTOCOL,
        })
        .unwrap();
        let opened = socket.clone();
        let _open = Closure::wrap(Box::new(move |_: BrowserEvent| {
            let _ = opened.send_with_str(&hello);
        }) as Box<dyn FnMut(_)>);
        socket.set_onopen(Some(_open.as_ref().unchecked_ref()));
        let received = events.clone();
        let specific = had_error.clone();
        let _message = Closure::wrap(Box::new(move |message: MessageEvent| {
            if let Some(text) = message.data().as_string() {
                if text.len() > 2_097_152 {
                    return;
                }
                match serde_json::from_str::<Event>(&text) {
                    Ok(Event::State(s)) if s.protocol != PROTOCOL => {
                        let _ = received.send(Event::Error(
                            "This server needs the current browser app. Reload the page.".into(),
                        ));
                    }
                    Ok(event) => {
                        if matches!(event, Event::Error(_)) {
                            specific.set(true);
                        }
                        let _ = received.send(event);
                    }
                    Err(_) => {
                        let _ = received.send(Event::Error(
                            "The server sent an incompatible message.".into(),
                        ));
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);
        socket.set_onmessage(Some(_message.as_ref().unchecked_ref()));
        let failed = events.clone();
        let fault = had_error.clone();
        let _error = Closure::wrap(Box::new(move |_: BrowserEvent| {
            if !fault.replace(true) {
                let _=failed.send(Event::Error("Could not reach this workspace. Check that the host is running, then try Join again.".into()));
            }
        }) as Box<dyn FnMut(_)>);
        socket.set_onerror(Some(_error.as_ref().unchecked_ref()));
        let _close = Closure::wrap(Box::new(move |_: BrowserEvent| {
            if !had_error.replace(true) {
                let _ = events.send(Event::Error(
                    "The host closed this workspace. Reconnect when the server is running again."
                        .into(),
                ));
            }
            let _ = events.send(Event::Disconnected);
        }) as Box<dyn FnMut(_)>);
        socket.set_onclose(Some(_close.as_ref().unchecked_ref()));
        Self {
            tx: Sender(socket.clone()),
            rx,
            socket,
            _open,
            _message,
            _error,
            _close,
        }
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.socket.set_onopen(None);
        self.socket.set_onmessage(None);
        self.socket.set_onerror(None);
        self.socket.set_onclose(None);
        let _ = self.socket.close();
    }
}
// Browser visitors never own a listener or an editor key. The UI hides hosting too.
pub struct Server {
    pub address: std::net::SocketAddr,
    pub editor_key: String,
}
impl Server {
    pub fn web_port(&self) -> Option<u16> {
        None
    }
    pub fn enable_web(&mut self, _: &str) -> Result<(), String> {
        Err("Hosting is available in the desktop app.".into())
    }
    pub fn start_with_rooms(
        _: &str,
        _: String,
        _: String,
        _: Vec<RoomMap>,
    ) -> Result<Self, String> {
        Err("Browser visitors join as guests. Create a server in the desktop app.".into())
    }
}
pub fn local_address() -> String {
    String::new()
}
