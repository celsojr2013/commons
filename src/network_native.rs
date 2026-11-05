use super::*;
#[cfg(test)]
use crate::world::default_world;
use crate::world::repair_targets;
#[path = "websocket_native.rs"]
mod websocket;
use std::collections::VecDeque;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};
impl Endpoint {
    fn resolve(&self) -> Result<Vec<std::net::SocketAddr>, String> {
        if self.is_local() {
            return Ok(vec![
                std::net::SocketAddr::from(([127, 0, 0, 1], self.port)),
                std::net::SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], self.port)),
            ]);
        }

        self.display()
            .to_socket_addrs()
            .map(|v| v.collect())
            .map_err(|_| format!("Could not find {}. Check the host address.", self.host))
    }
}
fn encode<T: Serialize>(v: &T) -> Vec<u8> {
    let mut b = serde_json::to_vec(v).unwrap();

    b.push(b'\n');

    b
}

fn flush(s: &mut TcpStream, out: &mut VecDeque<u8>) -> bool {
    while !out.is_empty() {
        let (a, _) = out.as_slices();

        match s.write(a) {
            Ok(0) => return false,
            Ok(n) => {
                out.drain(..n);
            }

            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(_) => return false,
        }
    }

    true
}

fn receive(s: &mut TcpStream, input: &mut Vec<u8>, limit: usize) -> bool {
    let mut b = [0u8; 8192];

    loop {
        match s.read(&mut b) {
            Ok(0) => return false,
            Ok(n) => {
                input.extend_from_slice(&b[..n]);

                if input.len() > limit {
                    return false;
                }
            }

            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return true,
            Err(_) => return false,
        }
    }
}

fn line<T: for<'a> Deserialize<'a>>(input: &mut Vec<u8>) -> Option<Result<T, serde_json::Error>> {
    let n = input.iter().position(|b| *b == b'\n')?;

    let bytes: Vec<u8> = input.drain(..=n).collect();

    Some(serde_json::from_slice(&bytes))
}

struct Peer {
    stream: TcpStream,
    input: Vec<u8>,
    output: VecDeque<u8>,
    person: Option<Person>,
    accepted: Instant,
    last_chat: Instant,
    can_edit: bool,
    last_admin_attempt: Option<Instant>,
    world_ready: bool,
    last_state: Vec<u8>,
    world_sent: Option<u64>,
    heartbeat: Instant,
    media_window: Instant,
    audio_count: u16,
    video_count: u16,
    screen_count: u16,
    video_subscription: Option<(Vec<u64>, Vec<u64>)>,
    scope: Option<TalkScope>,
}

pub struct Server {
    pub address: std::net::SocketAddr,
    pub editor_key: String,
    pub web: Option<crate::web_host::WebHost>,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

#[derive(Default)]
pub struct ServerOptions {
    pub admin_password: Option<String>,
    pub world_path: Option<std::path::PathBuf>,
}
impl Server {
    pub fn enable_web(&mut self, bind: &str) -> Result<(), String> {
        let ip = if self.address.ip().is_unspecified() {
            if self.address.is_ipv6() {
                std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)
            } else {
                std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
            }
        } else {
            self.address.ip()
        };
        let target = std::net::SocketAddr::new(ip, self.address.port());
        self.web = Some(crate::web_host::WebHost::start(bind, target)?);
        Ok(())
    }
    pub fn web_port(&self) -> Option<u16> {
        self.web.as_ref().map(|w| w.address.port())
    }

    #[cfg(test)]
    pub fn start(address: &str, name: String, password: String) -> Result<Self, String> {
        Self::start_with_rooms(address, name, password, default_world())
    }

    #[cfg(test)]
    pub fn start_with_rooms(
        address: &str,
        name: String,
        password: String,
        rooms: Vec<RoomMap>,
    ) -> Result<Self, String> {
        Self::start_configured(address, name, password, rooms, ServerOptions::default())
    }
    pub fn start_configured(
        address: &str,
        name: String,
        password: String,
        mut rooms: Vec<RoomMap>,
        options: ServerOptions,
    ) -> Result<Self, String> {
        if !crate::world::valid_world(&rooms) {
            return Err("The room designs are invalid.".into());
        }

        crate::world::normalize_world(&mut rooms);
        repair_targets(&mut rooms);

        let editor_key = rand::random::<[u8; 32]>()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();

        let admin = editor_key.clone();

        let listener=TcpListener::bind(address).map_err(|e|if e.kind()==std::io::ErrorKind::AddrInUse{
format!("Port {} is already in use. Join the existing local server or choose another port.",address.rsplit(':').next().unwrap_or("7777"))}
else{
format!("Could not start this server: {e}")}
)?;

        listener.set_nonblocking(true).map_err(|e| e.to_string())?;

        let address = listener.local_addr().map_err(|e| e.to_string())?;

        let stop = Arc::new(AtomicBool::new(false));

        let flag = stop.clone();

        let handle = thread::spawn(move || {
            let mut peers: Vec<Peer> = vec![];

            let mut state = Snapshot {
                space: name,
                rooms,
                world_revision: 1,
                protocol: PROTOCOL,
                ..Default::default()
            };

            let mut next_id = 1;

            let mut broadcast = Instant::now();

            while !flag.load(Ordering::Relaxed) {
                while let Ok((stream, _)) = listener.accept() {
                    if peers.len() >= 64 {
                        continue;
                    }

                    let _ = stream.set_nonblocking(true);

                    let _ = stream.set_nodelay(true);

                    peers.push(Peer {
                        stream,
                        input: vec![],
                        output: VecDeque::new(),
                        person: None,
                        accepted: Instant::now(),
                        last_chat: Instant::now() - Duration::from_secs(2),
                        can_edit: false,
                        last_admin_attempt: None,
                        world_ready: true,
                        last_state: vec![],
                        world_sent: None,
                        heartbeat: Instant::now(),
                        media_window: Instant::now(),
                        audio_count: 0,
                        video_count: 0,
                        screen_count: 0,
                        video_subscription: None,
                        scope: None,
                    });
                }

                let mut world_requests: Vec<(u64, Command)> = vec![];
                let mut chats: Vec<Message> = vec![];

                let mut media: Vec<(u64, TalkScope, MediaPacket)> = vec![];

                let mut audience: std::collections::HashMap<u64, Person> =
                    state.people.iter().cloned().map(|p| (p.id, p)).collect();
                peers.retain_mut(|p|{
if p.person.is_none()&&p.accepted.elapsed()>Duration::from_secs(8){
return false;
}
if !receive(&mut p.stream,&mut p.input,2_097_152){
return false;
}
if p.media_window.elapsed()>=Duration::from_secs(1){
p.media_window=Instant::now();
p.audio_count=0;
p.video_count=0;
p.screen_count=0;
}

 while let Some(cmd)=line::<Command>(&mut p.input){
match cmd{

 Ok(Command::Hello{
name,password:supplied,color,avatar,editor_key,protocol}
)if p.person.is_none()=>{
let error=if protocol!=PROTOCOL{
Some("This server needs the updated Commons app. Restart both apps with the latest prototype.")}
else if supplied!=password{
Some("The server password is incorrect.")}
else{
None}
;
if let Some(e)=error{
p.output.extend(encode(&Event::Error(e.into())));
let _=flush(&mut p.stream,&mut p.output);
return false;
}
let name:String=name.trim().chars().filter(|c|!c.is_control()).take(32).collect();
if name.is_empty(){
return false;
}
p.can_edit=editor_key==admin;
let spawn=state.rooms[0].spawn();
p.person=Some(Person{
id:next_id,name,room:0,x:spawn[0],y:spawn[1],color:color%6,away:false,avatar:if avatar.valid(){
avatar}
else{
Avatar::default()}
,direction:2,walking:false,mic:false,camera:false,screen:false}
);
p.output.extend(encode(&Event::Welcome(next_id)));
p.output.extend(encode(&Event::AdminStatus(p.can_edit)));
next_id+=1;
}

 Ok(Command::Move{
room,x,y}
)=>{
if let Some(person)=&mut p.person{
if p.world_ready&&room<state.rooms.len()&&x.is_finite()&&y.is_finite(){
person.room=room;
person.x=x.clamp(0.08,0.92);
person.y=y.clamp(0.18,0.88);
}
}
}

 Ok(Command::Motion{
room,x,y,direction,walking}
)=>{
if let Some(person)=&mut p.person{
if p.world_ready&&room<state.rooms.len()&&x.is_finite()&&y.is_finite(){
person.room=room;
person.x=x.clamp(0.001,0.999);
person.y=y.clamp(0.001,0.999);
person.direction=direction.min(3);
person.walking=walking;
}
}
}

 Ok(Command::Style(avatar))=>{
if avatar.valid(){
if let Some(person)=&mut p.person{
person.avatar=avatar;
}
}
}

 Ok(Command::AdminLogin { password: supplied }) => {
    if p.person.is_some() {
        if p.last_admin_attempt.is_some_and(|t| t.elapsed() < Duration::from_secs(2)) {
            p.output.extend(encode(&Event::Notice("Wait a moment before trying the administrator password again.".into())));
        } else {
            p.last_admin_attempt = Some(Instant::now());
            if options.admin_password.as_ref().is_some_and(|secret| !secret.is_empty() && secret == &supplied) {
                p.can_edit = true;
                p.output.extend(encode(&Event::Notice("Administration unlocked.".into())));
            } else {
                p.output.extend(encode(&Event::Notice("Administrator password was not accepted.".into())));
            }
            p.output.extend(encode(&Event::AdminStatus(p.can_edit)));
        }
    }
 }
 Ok(Command::AdminLogout) => { p.can_edit = false; p.output.extend(encode(&Event::AdminStatus(false))); }
 Ok(Command::WorldReady(revision)) => { if revision == state.world_revision { p.world_ready = true; } }
 Ok(command @ (Command::EditRoom { .. } | Command::CreateRoom { .. } | Command::DeleteRoom { .. })) => {
    if p.can_edit && p.world_ready {
        if let Some(person) = &p.person { world_requests.push((person.id, command)); }
    } else { p.output.extend(encode(&Event::Notice("Guests cannot edit maps. Unlock administrator access first.".into()))); }
 }

 Ok(Command::Away(v))=>{
if let Some(person)=&mut p.person{
person.away=v;
}
}

 Ok(Command::SubscribeVideo { cameras, screens }) => {
    p.video_subscription = Some((cameras.into_iter().take(16).collect(), screens.into_iter().take(2).collect()));
 }
 Ok(Command::ShareStatus(active)) => { if let Some(person) = &mut p.person { person.screen = active; } }
 Ok(Command::MediaStatus{
mic,camera}
)=>{
if let Some(person)=&mut p.person{
person.mic=mic;
person.camera=camera;
}
}

 Ok(Command::Chat{text,room,scope:expected})=>{
if let Some(person)=&p.person{
if p.world_ready&&p.last_chat.elapsed()>Duration::from_millis(150){
let text:String=text.trim().chars().filter(|c|!c.is_control()).take(1000).collect();
if !text.is_empty(){
let scope=person.scope(&state.rooms);
if expected.as_ref().is_some_and(|s|s!=&scope){p.output.extend(encode(&Event::Notice("The conversation changed before your message arrived, so it was not sent.".into())));continue;}
let private=scope.area.is_some();
chats.push(Message{
name:person.name.clone(),text,room:if private||room.is_some(){
Some(person.room)}
else{
None}
,area:scope.area,recipients:{let mut ids:Vec<u64>=audience.values().filter(|p|p.scope(&state.rooms)==scope).map(|p|p.id).collect();if !ids.contains(&person.id){ids.push(person.id);}ids},time:std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()}
);
p.last_chat=Instant::now();
}
}
}
}

 Ok(Command::Media(packet))=>{
if let Some(person)=&p.person{
let allowed=match packet.kind{
MediaKind::Audio=>{
p.audio_count=p.audio_count.saturating_add(1);
person.mic&&p.audio_count<=60}
,MediaKind::Video=>{
p.video_count=p.video_count.saturating_add(1);
person.camera&&p.video_count<=30}
,MediaKind::Screen=>{p.screen_count=p.screen_count.saturating_add(1);person.screen&&p.screen_count<=10}
}
;
if p.world_ready&&allowed&&packet.valid()&&packet.scope.as_ref()==Some(&person.scope(&state.rooms)){
media.push((person.id,person.scope(&state.rooms),packet));
}
}
}

 _=>return false,
 }
if let Some(person)=&p.person{audience.insert(person.id,person.clone());}

}
p.output.len()<MAX_SERVER_EVENT_BYTES&&flush(&mut p.stream,&mut p.output)}
);

                for (requester, command) in world_requests {
                    if !peers
                        .iter()
                        .any(|p| p.can_edit && p.person.as_ref().is_some_and(|p| p.id == requester))
                    {
                        continue;
                    }
                    let previous = state.rooms.clone();
                    let mut updated = previous.clone();
                    let result: Result<bool, String> = (|| {
                        match command {
                            Command::EditRoom { room, map } => {
                                let current =
                                    updated.get(room).ok_or("This map no longer exists.")?;
                                if map.id != current.id || !map.validate() {
                                    return Err(
                                        "Map changed or is invalid. Reopen the editor.".into()
                                    );
                                }
                                updated[room] = map;
                                repair_targets(&mut updated);
                                Ok(false)
                            }
                            Command::CreateRoom { name, revision } => {
                                if revision != state.world_revision {
                                    return Err(
                                        "Maps changed. Try again with the current list.".into()
                                    );
                                }
                                let name = name.trim();
                                if name.is_empty()
                                    || name.chars().count() > 40
                                    || updated.len() >= 128
                                {
                                    return Err(
                                        "Use a name of 1–40 characters (up to 128 maps).".into()
                                    );
                                }
                                let mut map = crate::world::preset(1, name, 26, 18);
                                // Random stable identity; deleted IDs are never intentionally reused.
                                loop {
                                    map.id = rand::random::<u64>() >> 11;
                                    if map.id != 0 && !updated.iter().any(|m| m.id == map.id) {
                                        break;
                                    }
                                }
                                updated.push(map);
                                Ok(true)
                            }
                            Command::DeleteRoom { id, revision } => {
                                if revision != state.world_revision {
                                    return Err(
                                        "Maps changed. Review the current list before deleting."
                                            .into(),
                                    );
                                }
                                crate::world::remove_room(&mut updated, id)?;
                                Ok(true)
                            }
                            _ => unreachable!(),
                        }
                    })();
                    let result = result.and_then(|structural| {
                        if let Some(path) = &options.world_path {
                            crate::storage::write(path, &updated)?;
                        }
                        Ok(structural)
                    });
                    match result {
                        Ok(structural) => {
                            if structural && updated.len() < previous.len() {
                                for peer in &mut peers {
                                    if let Some(person) = &mut peer.person {
                                        let id = previous[person.room].id;
                                        if let Some(index) = updated.iter().position(|m| m.id == id)
                                        {
                                            person.room = index;
                                        } else {
                                            person.room = 0;
                                            let pos = updated[0].spawn();
                                            person.x = pos[0];
                                            person.y = pos[1];
                                        }
                                        person.mic = false;
                                        person.camera = false;
                                        person.screen = false;
                                        peer.world_ready = false;
                                        peer.scope = None;
                                        peer.output.extend(encode(&Event::ConversationChanged(
                                            person.scope(&updated),
                                        )));
                                    }
                                }
                                // Private audiences remain unchanged; remove history of deleted maps.
                                state.messages.retain_mut(|message| {
                                    if let Some(index) = message.room {
                                        message.room = previous.get(index).and_then(|m| {
                                            updated.iter().position(|n| n.id == m.id)
                                        });
                                        message.room.is_some()
                                    } else {
                                        true
                                    }
                                });
                                chats.clear();
                                media.clear();
                            }
                            state.rooms = updated;
                            state.world_revision += 1;
                            if let Some(peer) = peers
                                .iter_mut()
                                .find(|p| p.person.as_ref().is_some_and(|p| p.id == requester))
                            {
                                peer.output.extend(encode(&Event::Notice(
                                    "Map changes saved on the server.".into(),
                                )));
                            }
                        }
                        Err(error) => {
                            if let Some(peer) = peers
                                .iter_mut()
                                .find(|p| p.person.as_ref().is_some_and(|p| p.id == requester))
                            {
                                peer.output.extend(encode(&Event::Notice(format!(
                                    "Map change was not saved: {error}"
                                ))));
                            }
                        }
                    }
                }
                for peer in &mut peers {
                    if let Some(person) = &mut peer.person {
                        let scope = person.scope(&state.rooms);
                        if peer.scope.as_ref().is_some_and(|old| old != &scope) {
                            person.mic = false;
                            person.camera = false;
                            person.screen = false;
                            peer.output
                                .extend(encode(&Event::ConversationChanged(scope.clone())));
                        }
                        peer.scope = Some(scope);
                    }
                }
                state.people = peers.iter().filter_map(|p| p.person.clone()).collect();

                for message in chats {
                    state.messages.push(message);

                    if state.messages.len() > 200 {
                        state.messages.remove(0);
                    }
                }

                for (sender, scope, packet) in media {
                    let Some(person) = state.people.iter().find(|p| p.id == sender) else {
                        continue;
                    };

                    if person.scope(&state.rooms) != scope
                        || match packet.kind {
                            MediaKind::Audio => !person.mic,
                            MediaKind::Video => !person.camera,
                            MediaKind::Screen => !person.screen,
                        }
                    {
                        continue;
                    }

                    let encoded = encode(&Event::Media {
                        sender,
                        scope: scope.clone(),
                        packet: packet.clone(),
                    });

                    for peer in &mut peers {
                        if peer
                            .person
                            .as_ref()
                            .is_some_and(|p| p.id != sender && p.scope(&state.rooms) == scope)
                            && peer.output.len() < 1048576
                            && peer
                                .video_subscription
                                .as_ref()
                                .is_none_or(|(cameras, screens)| match packet.kind {
                                    MediaKind::Audio => true,
                                    MediaKind::Video => cameras.contains(&sender),
                                    MediaKind::Screen => screens.contains(&sender),
                                })
                        {
                            peer.output.extend(encoded.clone());
                        }
                    }
                }

                if broadcast.elapsed() >= Duration::from_millis(40) {
                    for peer in &mut peers {
                        if let Some(person) = &peer.person {
                            let snapshot = Snapshot {
                                space: state.space.clone(),
                                people: state.people.clone(),
                                messages: state
                                    .messages
                                    .iter()
                                    .filter(|m| m.visible_to(person, &state.rooms))
                                    .cloned()
                                    .collect(),
                                rooms: if peer.world_sent == Some(state.world_revision) {
                                    vec![]
                                } else {
                                    state.rooms.clone()
                                },
                                world_revision: state.world_revision,
                                protocol: PROTOCOL,
                            };

                            let packet = encode(&Event::State(snapshot));

                            if packet != peer.last_state
                                || peer.heartbeat.elapsed() > Duration::from_secs(2)
                            {
                                peer.output.extend(packet.clone());

                                peer.last_state = packet;
                                peer.world_sent = Some(state.world_revision);

                                peer.heartbeat = Instant::now();
                            }
                        }
                    }

                    broadcast = Instant::now();
                }

                thread::sleep(Duration::from_millis(5));
            }
        });

        Ok(Self {
            address,
            editor_key,
            web: None,
            stop,
            handle: Some(handle),
        })
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.web.take();
        self.stop.store(true, Ordering::Relaxed);

        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

pub struct Client {
    pub tx: mpsc::Sender<Command>,
    pub rx: mpsc::Receiver<Event>,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Client {
    #[cfg(test)]
    pub fn connect(address: String, name: String, password: String, color: usize) -> Self {
        Self::connect_custom(
            address,
            name,
            password,
            color,
            Avatar::default(),
            String::new(),
        )
    }

    pub fn connect_custom(
        address: String,
        name: String,
        password: String,
        color: usize,
        avatar: Avatar,
        editor_key: String,
    ) -> Self {
        let (tx, commands) = mpsc::channel();

        let (events, rx) = mpsc::channel();

        let stop = Arc::new(AtomicBool::new(false));

        let flag = stop.clone();

        let handle = thread::spawn(move || {
            let result = (|| -> Result<(), String> {
                if ["http://", "https://", "ws://", "wss://"]
                    .iter()
                    .any(|p| address.starts_with(p))
                {
                    return websocket::run(
                        address,
                        Command::Hello {
                            name,
                            password,
                            color,
                            avatar,
                            editor_key,
                            protocol: PROTOCOL,
                        },
                        commands,
                        events.clone(),
                        flag.clone(),
                    );
                }
                let endpoint = Endpoint::parse(&address)?;

                let mut stream = None;

                for address in endpoint.resolve()? {
                    if flag.load(Ordering::Relaxed) {
                        return Ok(());
                    }

                    if let Ok(s) = TcpStream::connect_timeout(&address, Duration::from_millis(1200))
                    {
                        stream = Some(s);

                        break;
                    }
                }

                let mut stream=stream.ok_or_else(||if endpoint.is_local(){
format!("No Commons server is reachable at {}. Create a local server with Host here, or start it in another window first.",endpoint.display())}
else{
format!("Could not reach {}. Check that the host is running and that the address and port are reachable.",endpoint.display())}
)?;

                stream.set_nonblocking(true).map_err(|e| e.to_string())?;

                let _ = stream.set_nodelay(true);

                let mut output: VecDeque<u8> = encode(&Command::Hello {
                    name,
                    password,
                    color,
                    avatar,
                    editor_key,
                    protocol: PROTOCOL,
                })
                .into();

                let mut input = vec![];
                let mut world_cache = super::world_sync::WorldCache::default();

                while !flag.load(Ordering::Relaxed) {
                    for cmd in commands.try_iter() {
                        if matches!(cmd, Command::Media(_)) && output.len() > 1048576 {
                            continue;
                        }

                        output.extend(encode(&cmd));
                    }

                    if !flush(&mut stream, &mut output) {
                        return Err("The server connection closed.".into());
                    }

                    let alive = receive(&mut stream, &mut input, MAX_SERVER_EVENT_BYTES);

                    while let Some(event) = line::<Event>(&mut input) {
                        match event{
Ok(Event::State(s))if s.protocol!=PROTOCOL=>return Err("That server is an older Commons prototype. Stop it and create a server with the updated app.".into()),Ok(mut event)=>{
if let Event::State(ref mut state)=event{world_cache.hydrate(state);}
let failed=matches!(event,Event::Error(_));
let _=events.send(event);
if failed{
return Ok(());
}
}
,Err(_)=>return Err("That address did not reply as a compatible Commons server.".into())}
                    }

                    if !alive {
                        return Err(
                            "The server went offline. You can reconnect from Servers.".into()
                        );
                    }

                    thread::sleep(Duration::from_millis(5));
                }

                Ok(())
            })();

            if let Err(e) = result {
                let _ = events.send(Event::Error(e));
            }

            let _ = events.send(Event::Disconnected);
        });

        Self {
            tx,
            rx,
            stop,
            handle: Some(handle),
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);

        if let Some(h) = self.handle.take() {
            if h.is_finished() {
                let _ = h.join();
            }
        }
    }
}

pub fn local_address() -> String {
    (|| {
        let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;

        s.connect("192.0.2.1:80").ok()?;

        Some(s.local_addr().ok()?.ip().to_string())
    })()
    .unwrap_or_else(|| "127.0.0.1".into())
}

#[cfg(test)]
mod tests {

    use super::*;

    fn wait(client: &Client, predicate: impl Fn(&Event) -> bool) -> Event {
        let end = Instant::now() + Duration::from_secs(4);

        loop {
            let e = client
                .rx
                .recv_timeout(end.saturating_duration_since(Instant::now()))
                .expect("network event timeout");

            if predicate(&e) {
                return e;
            }
        }
    }

    #[test]
    fn two_users_share_chat_movement_and_disconnect() {
        let server = Server::start("127.0.0.1:0", "Team room".into(), "secret".into()).unwrap();

        let a = Client::connect(
            server.address.to_string(),
            "Alice".into(),
            "secret".into(),
            0,
        );

        let b = Client::connect(server.address.to_string(), "Bob".into(), "secret".into(), 1);

        wait(
            &a,
            |e| matches!(e, Event::State(s) if s.people.len() == 2 && s.space == "Team room"),
        );

        b.tx.send(Command::Move {
            room: 2,
            x: 0.3,
            y: 0.4,
        })
        .unwrap();

        b.tx.send(Command::Chat {
            text: "Hello Alice".into(),
            room: None,
            scope: None,
        })
        .unwrap();

        wait(
            &a,
            |e| matches!(e, Event::State(s) if s.people.iter().any(|p| p.name == "Bob" && p.room == 2 && p.x == 0.3) && s.messages.iter().any(|m| m.text == "Hello Alice" && m.name == "Bob")),
        );

        drop(b);

        wait(&a, |e| matches!(e, Event::State(s) if s.people.len() == 1));

        drop(server);

        wait(&a, |e| matches!(e, Event::Error(_)));
    }

    #[test]
    fn rejects_wrong_password_and_invalid_positions() {
        let server = Server::start("127.0.0.1:0", "Private".into(), "secret".into()).unwrap();

        let bad = Client::connect(
            server.address.to_string(),
            "Guest".into(),
            "wrong".into(),
            1,
        );

        wait(
            &bad,
            |e| matches!(e, Event::Error(s) if s.contains("password")),
        );

        let good = Client::connect(
            server.address.to_string(),
            "Host".into(),
            "secret".into(),
            0,
        );

        wait(&good, |e| matches!(e, Event::Welcome(_)));

        good.tx
            .send(Command::Move {
                room: 0,
                x: 100.0,
                y: -100.0,
            })
            .unwrap();

        wait(
            &good,
            |e| matches!(e, Event::State(s) if s.people.len() == 1 && s.people[0].x == 0.92 && s.people[0].y == 0.18),
        );
    }

    #[test]
    fn host_designs_and_live_outfits_are_shared_but_guest_edits_are_rejected() {
        let server = Server::start("127.0.0.1:0", "Design studio".into(), "".into()).unwrap();

        let host = Client::connect_custom(
            server.address.to_string(),
            "Host".into(),
            "".into(),
            0,
            Avatar::default(),
            server.editor_key.clone(),
        );

        let guest = Client::connect(server.address.to_string(), "Guest".into(), "".into(), 0);

        wait(&guest, |e| matches!(e,Event::State(s) if s.people.len()==2));

        let mut design = crate::world::preset(2, "Our forest", 32, 24);
        design.id = 2;

        design.stamp(45, 4, 4, true);

        host.tx
            .send(Command::EditRoom {
                room: 1,
                map: design.clone(),
            })
            .unwrap();

        wait(
            &guest,
            |e| matches!(e,Event::State(s) if s.world_revision==2 && s.rooms[1]==design),
        );

        let look = Avatar {
            body: 1,
            bottom: 3,
            shirt: 5,
            skin: 5,
            hair: 9,
            accessory: 6,
            ..Avatar::default()
        };

        guest.tx.send(Command::Style(look.clone())).unwrap();

        guest
            .tx
            .send(Command::Motion {
                room: 1,
                x: 0.6,
                y: 0.7,
                direction: 3,
                walking: true,
            })
            .unwrap();

        wait(
            &host,
            |e| matches!(e,Event::State(s) if s.people.iter().any(|p|p.name=="Guest" && p.avatar==look && p.direction==3 && p.walking && p.room==1)),
        );

        let mut rejected = design.clone();

        rejected.name = "Guest overwrite".into();

        guest
            .tx
            .send(Command::EditRoom {
                room: 1,
                map: rejected,
            })
            .unwrap();

        let mut broken = design.clone();

        broken.ground.clear();

        host.tx
            .send(Command::EditRoom {
                room: 1,
                map: broken,
            })
            .unwrap();

        // A later chat snapshot confirms both commands were processed without changing the room.
        guest
            .tx
            .send(Command::Chat {
                text: "After edit attempts".into(),
                room: None,
                scope: None,
            })
            .unwrap();

        wait(
            &host,
            |e| matches!(e,Event::State(s) if s.messages.iter().any(|m|m.text=="After edit attempts") && s.world_revision==2 && s.rooms[1]==design),
        );
    }

    #[test]
    fn local_address_normalization_and_both_loopback_families_connect() {
        for value in [
            "localhost:7777",
            "commons://localhost:7777/",
            "http://localhost:7777/",
            "localhost",
        ] {
            let e = Endpoint::parse(value).unwrap();
            assert_eq!(e.host, "localhost");
            assert_eq!(e.port, 7777);
        }

        assert!(Endpoint::parse("localhost:0").is_err());
        assert!(Endpoint::parse("http://localhost:bad").is_err());
        assert!(Endpoint::parse("localhost:7777/room").is_err());

        let server = Server::start("127.0.0.1:0", "Loopback".into(), "".into()).unwrap();

        for host in ["localhost", "[::1]", "127.0.0.1"] {
            let client = Client::connect(
                format!("{host}:{}", server.address.port()),
                "Guest".into(),
                "".into(),
                0,
            );
            wait(&client, |e| matches!(e, Event::Welcome(_)));
        }

        assert!(
            Server::start(&server.address.to_string(), "Duplicate".into(), "".into())
                .err()
                .unwrap()
                .contains("already in use")
        );
    }

    fn no_media_until_marker(client: &Client, away: bool) {
        let until = Instant::now() + Duration::from_secs(4);
        loop {
            let event = client
                .rx
                .recv_timeout(until.saturating_duration_since(Instant::now()))
                .unwrap();
            assert!(
                !matches!(event, Event::Media { .. }),
                "media leaked to a different conversation"
            );
            if matches!(event,Event::State(s) if s.people.iter().any(|p|p.name=="Alice"&&p.away==away))
            {
                break;
            }
        }
    }

    #[test]
    fn private_chat_audio_and_video_never_reach_outsiders_and_stop_on_leaving() {
        let mut rooms = default_world();
        rooms[0] = crate::world::RoomMap::blank("Privacy test", 12, 12, 0);
        rooms[0].areas.push(crate::world::PrivateArea {
            id: 77,
            name: "Private".into(),
            x: 2,
            y: 2,
            width: 4,
            height: 4,
        });

        crate::world::normalize_world(&mut rooms);
        let server =
            Server::start_with_rooms("127.0.0.1:0", "Privacy".into(), "".into(), rooms).unwrap();

        let a = Client::connect(server.address.to_string(), "Alice".into(), "".into(), 0);
        let b = Client::connect(server.address.to_string(), "Bob".into(), "".into(), 0);
        let outside = Client::connect(server.address.to_string(), "Outside".into(), "".into(), 0);

        for c in [&a, &b, &outside] {
            wait(c, |e| matches!(e, Event::Welcome(_)));
        }

        for c in [&a, &b] {
            c.tx.send(Command::Motion {
                room: 0,
                x: 0.3,
                y: 0.3,
                direction: 2,
                walking: false,
            })
            .unwrap();
        }

        outside
            .tx
            .send(Command::Motion {
                room: 0,
                x: 0.85,
                y: 0.85,
                direction: 2,
                walking: false,
            })
            .unwrap();

        wait(
            &outside,
            |e| matches!(e,Event::State(s) if s.people.iter().filter(|p|p.scope(&s.rooms).area==Some(77)).count()==2),
        );

        a.tx.send(Command::Chat {
            text: "PRIVATE_SENTINEL".into(),
            room: None,
            scope: None,
        })
        .unwrap();

        wait(
            &b,
            |e| matches!(e,Event::State(s) if s.messages.iter().any(|m|m.text=="PRIVATE_SENTINEL"&&m.area==Some(77))),
        );

        a.tx.send(Command::MediaStatus {
            mic: true,
            camera: true,
        })
        .unwrap();
        let audio =
            MediaPacket::from_bytes(MediaKind::Audio, &vec![0; 640]).with_scope(TalkScope {
                room: 0,
                area: Some(77),
            });
        a.tx.send(Command::Media(audio.clone())).unwrap();
        let video = MediaPacket::from_bytes(MediaKind::Video, &[255, 216, 255, 217]).with_scope(
            TalkScope {
                room: 0,
                area: Some(77),
            },
        );
        a.tx.send(Command::Media(video)).unwrap();
        a.tx.send(Command::Away(true)).unwrap();

        wait(&b, |e| {
            matches!(e,Event::Media{
scope,packet,..}
 if scope.area==Some(77)&&packet.kind==MediaKind::Audio)
        });
        wait(&b, |e| {
            matches!(e,Event::Media{
scope,packet,..}
 if scope.area==Some(77)&&packet.kind==MediaKind::Video)
        });

        no_media_until_marker(&outside, true);

        let state = wait(
            &outside,
            |e| matches!(e,Event::State(s) if s.people.iter().any(|p|p.name=="Alice"&&p.away)),
        );

        assert!(!serde_json::to_string(&state)
            .unwrap()
            .contains("PRIVATE_SENTINEL"));

        b.tx.send(Command::Motion {
            room: 0,
            x: 0.8,
            y: 0.8,
            direction: 2,
            walking: false,
        })
        .unwrap();

        let left = wait(
            &b,
            |e| matches!(e,Event::State(s) if s.people.iter().any(|p|p.name=="Bob"&&p.x==0.8)),
        );

        assert!(!serde_json::to_string(&left)
            .unwrap()
            .contains("PRIVATE_SENTINEL"));

        a.tx.send(Command::Media(audio)).unwrap();
        a.tx.send(Command::Away(false)).unwrap();
        no_media_until_marker(&b, false);
    }
    #[test]
    fn editing_an_active_private_area_pauses_media_and_rejects_old_scope_envelopes() {
        let mut rooms = default_world();
        rooms[0] = RoomMap::blank("Active call", 12, 12, 0);
        rooms[0].areas.push(crate::world::PrivateArea {
            id: 77,
            name: "Private".into(),
            x: 2,
            y: 2,
            width: 4,
            height: 4,
        });
        crate::world::normalize_world(&mut rooms);
        let server =
            Server::start_with_rooms("127.0.0.1:0", "Live edit".into(), "".into(), rooms.clone())
                .unwrap();
        let a = Client::connect_custom(
            server.address.to_string(),
            "Alice".into(),
            "".into(),
            0,
            Avatar::default(),
            server.editor_key.clone(),
        );
        let outside = Client::connect(server.address.to_string(), "Outside".into(), "".into(), 0);
        wait(&a, |e| matches!(e, Event::Welcome(_)));
        wait(&outside, |e| matches!(e, Event::Welcome(_)));
        a.tx.send(Command::Motion {
            room: 0,
            x: 0.3,
            y: 0.3,
            direction: 2,
            walking: false,
        })
        .unwrap();
        wait(
            &a,
            |e| matches!(e,Event::State(s) if s.people.iter().any(|p|p.name=="Alice"&&p.scope(&s.rooms).area==Some(77))),
        );
        a.tx.send(Command::MediaStatus {
            mic: true,
            camera: true,
        })
        .unwrap();
        wait(
            &outside,
            |e| matches!(e,Event::State(s) if s.people.iter().any(|p|p.name=="Alice"&&p.mic&&p.camera)),
        );
        let old = TalkScope {
            room: 0,
            area: Some(77),
        };
        rooms[0].areas.clear();
        a.tx.send(Command::EditRoom {
            room: 0,
            map: rooms[0].clone(),
        })
        .unwrap();
        a.tx.send(Command::Chat {
            text: "STALE_PRIVATE_TEXT".into(),
            room: Some(0),
            scope: Some(old.clone()),
        })
        .unwrap();
        a.tx.send(Command::Media(
            MediaPacket::from_bytes(MediaKind::Audio, &vec![0; 640]).with_scope(old.clone()),
        ))
        .unwrap();
        a.tx.send(Command::Media(
            MediaPacket::from_bytes(MediaKind::Video, &[255, 216, 255, 217])
                .with_scope(old.clone()),
        ))
        .unwrap();
        a.tx.send(Command::Away(true)).unwrap();
        no_media_until_marker(&outside, true);
        wait(
            &a,
            |e| matches!(e,Event::ConversationChanged(scope) if scope.area.is_none()),
        );
        let state = wait(&a, |e| matches!(e,Event::State(s) if s.world_revision==2));
        assert!(
            matches!(&state,Event::State(s) if s.people.iter().any(|p|p.name=="Alice"&&!p.mic&&!p.camera))
        );
        assert!(!serde_json::to_string(&state)
            .unwrap()
            .contains("STALE_PRIVATE_TEXT"));
        // Even a re-enabled sender cannot re-label a buffered private packet as public.
        a.tx.send(Command::MediaStatus {
            mic: true,
            camera: true,
        })
        .unwrap();
        a.tx.send(Command::Media(
            MediaPacket::from_bytes(MediaKind::Audio, &vec![0; 640]).with_scope(old),
        ))
        .unwrap();
        a.tx.send(Command::Away(false)).unwrap();
        no_media_until_marker(&outside, false);
    }
    #[test]
    fn expanding_a_private_area_does_not_reveal_earlier_messages_to_new_members() {
        use std::io::BufRead;
        let mut rooms = default_world();
        rooms[0] = RoomMap::blank("Area", 12, 12, 0);
        rooms[0].areas.push(crate::world::PrivateArea {
            id: 77,
            name: "Private".into(),
            x: 2,
            y: 2,
            width: 4,
            height: 4,
        });
        crate::world::normalize_world(&mut rooms);
        let server =
            Server::start_with_rooms("127.0.0.1:0", "Audience".into(), "".into(), rooms.clone())
                .unwrap();
        let mut owner = TcpStream::connect(server.address).unwrap();
        owner
            .set_read_timeout(Some(Duration::from_secs(4)))
            .unwrap();
        owner.set_nodelay(true).unwrap();
        owner
            .write_all(&encode(&Command::Hello {
                name: "Owner".into(),
                password: "".into(),
                color: 0,
                avatar: Avatar::default(),
                editor_key: server.editor_key.clone(),
                protocol: PROTOCOL,
            }))
            .unwrap();
        let mut reader = std::io::BufReader::new(owner.try_clone().unwrap());
        let mut raw = String::new();
        reader.read_line(&mut raw).unwrap();
        assert!(matches!(
            serde_json::from_str::<Event>(&raw).unwrap(),
            Event::Welcome(_)
        ));
        let outside = Client::connect(
            server.address.to_string(),
            "New member".into(),
            "".into(),
            0,
        );
        wait(&outside, |e| matches!(e, Event::Welcome(_)));
        owner
            .write_all(&encode(&Command::Motion {
                room: 0,
                x: 0.3,
                y: 0.3,
                direction: 2,
                walking: false,
            }))
            .unwrap();
        wait(
            &outside,
            |e| matches!(e,Event::State(s) if s.people.iter().any(|p|p.name=="Owner"&&p.scope(&s.rooms).area==Some(77))),
        );
        let chat = Command::Chat {
            text: "EARLIER_PRIVATE_HISTORY".into(),
            room: Some(0),
            scope: Some(TalkScope {
                room: 0,
                area: Some(77),
            }),
        };
        rooms[0].areas[0].x = 0;
        rooms[0].areas[0].y = 0;
        rooms[0].areas[0].width = 12;
        rooms[0].areas[0].height = 12;
        let mut combined = encode(&chat);
        combined.extend(encode(&Command::EditRoom {
            room: 0,
            map: rooms[0].clone(),
        }));
        combined.extend(encode(&Command::Away(true)));
        assert!(combined.len() < 1400);
        owner.write_all(&combined).unwrap();
        let state = wait(
            &outside,
            |e| matches!(e,Event::State(s) if s.world_revision==2&&s.people.iter().all(|p|p.scope(&s.rooms).area==Some(77))),
        );
        assert!(!serde_json::to_string(&state)
            .unwrap()
            .contains("EARLIER_PRIVATE_HISTORY"));
        loop {
            raw.clear();
            reader.read_line(&mut raw).unwrap();
            if let Event::State(s) = serde_json::from_str::<Event>(&raw).unwrap() {
                if s.world_revision == 2 {
                    assert!(s
                        .messages
                        .iter()
                        .any(|m| m.text == "EARLIER_PRIVATE_HISTORY"));
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod administration_tests {
    use super::*;
    fn wait(client: &Client, predicate: impl Fn(&Event) -> bool) -> Event {
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            let event = client
                .rx
                .recv_timeout(end.saturating_duration_since(Instant::now()))
                .expect("event deadline");
            if predicate(&event) {
                return event;
            }
        }
    }
    fn state(client: &Client, revision: u64) -> Snapshot {
        match wait(
            client,
            |e| matches!(e, Event::State(s) if s.world_revision == revision),
        ) {
            Event::State(s) => s,
            _ => unreachable!(),
        }
    }
    #[test]
    fn web_admin_can_create_delete_and_persist_maps_but_guests_cannot() {
        let path =
            std::env::temp_dir().join(format!("commons-world-test-{}.json", rand::random::<u64>()));
        let mut server = Server::start_configured(
            "127.0.0.1:0",
            "Admin test".into(),
            "join".into(),
            crate::world::initial_world(),
            ServerOptions {
                admin_password: Some("admin-test-secret".into()),
                world_path: Some(path.clone()),
            },
        )
        .unwrap();
        server.enable_web("127.0.0.1:0").unwrap();
        let address = format!("http://127.0.0.1:{}", server.web_port().unwrap());
        let admin = Client::connect_custom(
            address.clone(),
            "Admin".into(),
            "join".into(),
            0,
            Avatar::default(),
            server.editor_key.clone(),
        );
        wait(&admin, |e| matches!(e, Event::AdminStatus(false))); // Web handshake cannot grant privilege.
        let guest = Client::connect(address, "Guest".into(), "join".into(), 0);
        wait(&guest, |e| matches!(e, Event::Welcome(_)));
        guest
            .tx
            .send(Command::AdminLogin {
                password: "wrong".into(),
            })
            .unwrap();
        wait(
            &guest,
            |e| matches!(e, Event::Notice(s) if s.contains("not accepted")),
        );
        guest
            .tx
            .send(Command::CreateRoom {
                name: "Forbidden".into(),
                revision: 1,
            })
            .unwrap();
        wait(
            &guest,
            |e| matches!(e, Event::Notice(s) if s.contains("cannot edit")),
        );
        assert!(!path.exists());
        admin
            .tx
            .send(Command::AdminLogin {
                password: "admin-test-secret".into(),
            })
            .unwrap();
        wait(&admin, |e| matches!(e, Event::AdminStatus(true)));
        admin
            .tx
            .send(Command::CreateRoom {
                name: "Review".into(),
                revision: 1,
            })
            .unwrap();
        let added = state(&admin, 2);
        assert_eq!(added.rooms.len(), 2);
        assert_ne!(added.rooms[0].id, added.rooms[1].id);
        let stable_id = added.rooms[1].id;
        guest
            .tx
            .send(Command::Move {
                room: 1,
                x: 0.5,
                y: 0.5,
            })
            .unwrap();
        wait(
            &admin,
            |e| matches!(e, Event::State(s) if s.people.iter().any(|p| p.name=="Guest" && p.room==1)),
        );
        admin
            .tx
            .send(Command::DeleteRoom {
                id: added.rooms[0].id,
                revision: 2,
            })
            .unwrap();
        let removed = state(&admin, 3);
        assert_eq!(removed.rooms.len(), 1);
        assert_eq!(removed.rooms[0].id, stable_id);
        assert!(removed
            .people
            .iter()
            .all(|p| p.room == 0 && !p.mic && !p.camera && !p.screen));
        let saved: Vec<RoomMap> = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(saved, removed.rooms);
        admin.tx.send(Command::WorldReady(3)).unwrap();
        admin
            .tx
            .send(Command::DeleteRoom {
                id: stable_id,
                revision: 3,
            })
            .unwrap();
        wait(
            &admin,
            |e| matches!(e, Event::Notice(s) if s.contains("Keep one entrance")),
        );
        admin.tx.send(Command::AdminLogout).unwrap();
        wait(&admin, |e| matches!(e, Event::AdminStatus(false)));
        admin
            .tx
            .send(Command::CreateRoom {
                name: "No longer allowed".into(),
                revision: 3,
            })
            .unwrap();
        wait(
            &admin,
            |e| matches!(e, Event::Notice(s) if s.contains("cannot edit")),
        );
        drop(guest);
        drop(admin);
        drop(server);
        let restarted =
            Server::start_with_rooms("127.0.0.1:0", "Restored".into(), "".into(), saved.clone())
                .unwrap();
        let client = Client::connect(restarted.address.to_string(), "Reader".into(), "".into(), 0);
        assert_eq!(state(&client, 1).rooms, saved);
        drop(client);
        drop(restarted);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn screen_and_camera_are_independent_and_video_subscription_filters_delivery() {
        let server = Server::start("127.0.0.1:0", "Streams".into(), "".into()).unwrap();
        let sender = Client::connect(server.address.to_string(), "Presenter".into(), "".into(), 0);
        let viewer = Client::connect(server.address.to_string(), "Viewer".into(), "".into(), 0);
        let id = match wait(&sender, |e| matches!(e, Event::Welcome(_))) {
            Event::Welcome(id) => id,
            _ => unreachable!(),
        };
        wait(
            &viewer,
            |e| matches!(e, Event::State(s) if s.people.len()==2),
        );
        viewer
            .tx
            .send(Command::SubscribeVideo {
                cameras: vec![],
                screens: vec![id],
            })
            .unwrap();
        viewer.tx.send(Command::Away(true)).unwrap();
        wait(
            &sender,
            |e| matches!(e, Event::State(s) if s.people.iter().any(|p|p.name=="Viewer"&&p.away)),
        );
        sender
            .tx
            .send(Command::MediaStatus {
                mic: false,
                camera: true,
            })
            .unwrap();
        sender.tx.send(Command::ShareStatus(true)).unwrap();
        wait(
            &viewer,
            |e| matches!(e, Event::State(s) if s.people.iter().any(|p|p.id==id&&p.camera&&p.screen)),
        );
        let scope = TalkScope {
            room: 0,
            area: None,
        };
        sender
            .tx
            .send(Command::Media(
                MediaPacket::from_bytes(MediaKind::Video, &[255, 216, 255, 217])
                    .with_scope(scope.clone()),
            ))
            .unwrap();
        sender
            .tx
            .send(Command::Media(
                MediaPacket::from_bytes(MediaKind::Screen, &[255, 216, 255, 217])
                    .with_scope(scope.clone()),
            ))
            .unwrap();
        let end = Instant::now() + Duration::from_secs(3);
        loop {
            let event = viewer
                .rx
                .recv_timeout(end.saturating_duration_since(Instant::now()))
                .unwrap();
            if let Event::Media { packet, .. } = event {
                assert_eq!(packet.kind, MediaKind::Screen);
                break;
            }
        }
        sender.tx.send(Command::ShareStatus(false)).unwrap();
        wait(
            &viewer,
            |e| matches!(e, Event::State(s) if s.people.iter().any(|p|p.id==id&&p.camera&&!p.screen)),
        );
        sender.tx.send(Command::ShareStatus(true)).unwrap();
        sender
            .tx
            .send(Command::Move {
                room: 1,
                x: 0.5,
                y: 0.5,
            })
            .unwrap();
        wait(
            &viewer,
            |e| matches!(e, Event::State(s) if s.people.iter().any(|p|p.id==id&&p.room==1&&!p.camera&&!p.screen)),
        );
    }
}
