# Commons desktop — pixel workspace studio

A self-hosted virtual workspace built with Rust / eframe, with desktop and browser clients, editable tile maps, animated characters, chat, audio, video and screen sharing.

## Meetings and administration · 0.5.0

Protocol 5: deploy the matching server and clients together. New workspaces begin with one map; existing valid world files keep their maps and gain stable map IDs.

- **Profile → Administration** accepts a separate `COMMONS_ADMIN_PASSWORD`. Administrators can create/delete maps and use **Edit** in web and desktop sessions. The server authorizes every mutation and atomically saves `world.json` before publishing changes. Deleting an occupied map relocates participants to the entrance; incoming doors are removed and other door targets are repaired. One entrance map must remain. Administrative access ends on logout or disconnect. The shared entry password remains `COMMONS_PASSWORD`.
- **Call → Meeting view** opens a large meeting layout; **Map view** returns without changing conversation or position. Camera tiles are paginated in groups of six. Click a camera or a named shared screen to pin it locally. The first available screen is selected by default; subsequent presenters do not replace an existing selection. **Gallery** clears the pin.
- **Share screen** operates independently of the webcam. Web clients use their browser's screen/window/tab selector; desktop clients list displays and windows through the OS capture backend. Several people can present at once, and each viewer chooses one screen. Screen capture ends when the conversation changes, the source ends, or the user stops sharing. No system/tab audio is captured in this version; microphone audio remains independent.
- **Devices → Camera quality** offers Low (320×240, up to 6 fps), Medium (640×480, up to 15 fps), and High (1280×720, up to 24 fps). Screen sharing preserves aspect ratio up to 1920×1080 at up to 5 fps, prioritizing text readability. Actual FPS depends on the camera, browser, machine and network. Audio remains PCM and video JPEG over the existing transport; this is not a WebRTC conference service.
- Receivers subscribe to visible camera tiles and the selected screen, reducing off-page video delivery. Streams continue to respect room/private-area membership. Admin access does not bypass private media boundaries.

Use HTTPS for browser capture. Native capture requires the operating system's screen-recording permission; Linux Wayland behavior depends on the compositor/portal. macOS native compilation and browser synthetic capture are checked separately from physical-device acceptance. Desktop releases for Windows/Linux need platform-specific packaging and hardware validation.

See [hosting instructions](deploy/README.md) for self-hosting and [implementation notes](docs/IMPLEMENTATION.md) for validation scope and remaining platform checks.

## Make a space

Click your profile in the bottom-left corner, choose **Create server**, enter a name and workspace name, and create it. **Profile → Server & invite** gives you an invite to copy. Other people choose **Profile → Join / switch server** and paste the address. Creating a server also joins you automatically. Joining that same localhost address again reuses the connection instead of reporting an error. **Host here** on the Join screen can start a missing local server. Localhost, IPv4/IPv6 loopback, and pasted Commons/HTTP-style addresses are normalized.

Your computer hosts while the app is open. A server supports up to 64 connections; its latest 200 chat messages are kept for the session.

Local-network guests can normally use the displayed LAN address. Internet guests need a publicly reachable address and incoming access to the selected port (7777 by default). There is no hosted relay or automatic router configuration. Raw TCP and local HTTP use unencrypted transport; use trusted networks or deploy the browser endpoint behind HTTPS as described below. Closing the app or stopping the server disconnects guests.

## Build your rooms

Choose **Edit** beside **ROOMS**. Local users, the server host, and authenticated administrators can edit; other guests see the shared design.

- Rename rooms and set each map between 12×12 and 64×64 tiles.
- Choose from 51 brushes: floors, walls, furniture, nature, and friendly Halloween decorations.
- Drag to paint floors or walls. Fill changes connected floor tiles. Click to stamp props; right-click or Erase removes props.
- Toggle whether newly placed props block walking. Movement uses collision checks and paths around props.
- Undo and redo strokes; Command/Ctrl+Z undoes and Shift+Command/Ctrl+Z redoes while the canvas has focus.
- Resize with **Apply size**. Shrinking crops out-of-bounds content; Undo restores it.
- Start with Halloween office, cozy office, forest garden, or blank canvas themes.
- Save and load named designs in your local library. **Save room** publishes the draft to connected guests and stores the world locally; **Cancel** discards the draft.

Click clear floor to walk, or use arrows/WASD. Use the zoom slider for detail. Hold **Alt** and use the mouse wheel over the canvas to zoom around your pointer. The **+ / −** buttons and slider also zoom. Scroll, middle-drag, or Space-drag to pan; **Fit** restores the view. While walking with the keyboard or a floor click, the camera tracks your character near the viewport edges. Manual panning stays where you leave it while idle, and tracking resumes when you move. Room editing keeps free panning. Open **People** to see who is in each room and visit them. Set yourself away and chat with everyone or just the current room.

## Make your character

Choose **Profile → Avatar & profile**. Select masculine or feminine bodies, eight skin palettes, three expressions, ten hairstyles with eight colors, six tops per body, trousers/shorts/skirts, and seven accessory options. Preview idle/walk animation and rotate through four directions. **Save my character** updates your profile and sends your look to connected users without reconnecting. Display names can be changed while disconnected.

**Export my spritesheet** writes a new PNG and accompanying credits/license into the app's exports folder; **Open exports** opens it. Sheets are 576×512 with 64×64 frames: rows 0–3 are up/left/down/right walking with nine columns (column 0 is the standing pose); rows 4–7 contain two idle frames in columns 0–1. Pixel textures use nearest-neighbor filtering.

Worlds, named designs, and profiles persist locally. Passwords are not saved. Art sources, author-specific attribution, license texts, downloaded layers and hashes are shipped in assets and discoverable through **Art & credits** inside the app. Environment artwork uses Kenney CC0 packs plus original Commons tiles and decorations. Character artwork comes from the Universal LPC catalog, with individual source licenses preserved and composed/recolored sheets offered under CC-BY-SA 4.0. No original generator code is used.

## Build and check

Run `cargo build`, `cargo test`, and `cargo clippy --all-targets -- -D warnings` in this directory. `cargo run` starts the app.

`cargo run -- --screenshot shots/pixel-office.png` renders 120 frames, saves a native PNG and exits. Add `--view editor`, `--view avatar`, or `--view credits` to inspect those states. These runs don't overwrite the user's settings on exit.

Automated tests cover networking, server-authorized room publication, live avatar updates, rejected guest edits and invalid maps, collision/pathfinding, resize/fill/erase, undo/redo, atomic save/reload, real pointer interaction with the editors, and visible animation frames for all appearance categories.

Importing arbitrary tilesets and hosted internet relays are outside this version.


## Private areas and doors

In **Edit room**, choose **Areas & doors** from the palette category.

**Private area** places a named rectangle on the map with your chosen width/height. Click an existing area or select it in the list to update or remove it. Areas must fit inside the map and cannot overlap. The server computes conversation membership from player positions. Inside an area, text, audio and video go only to matching participants. Private text history is restricted to the participants present when the server accepted the message, and is hidden when they leave; a newly arriving member does not receive earlier private history.

Voice/video stay within the current map room outside private areas. Room chat likewise stays in its room; **Everyone** is the explicit workspace-wide text channel outside private areas. Inside a private area, chat is automatically private.

**Door** places a transition on clear floor. Select a destination room and arrival tile, or choose **Use room entrance**. Walk onto a door to travel. Arrivals are latched until you step away, preventing immediate bounce loops. A door is one-way; place another door if you want a return route. Existing room designs are preserved; new workspaces start with one room, and you can add maps, areas and doors.

The host and authenticated administrators can publish these changes to guests. The server pauses mic, camera and screen sharing on conversation changes, including area edits during a call. Chat/media packets carry their intended conversation; a stale private packet cannot be delivered to a public conversation. Previously accepted private messages keep their original audience even if the host later expands an area.

## Camera, microphone and speaker controls

Use the bottom toolbar: **Mic on/off**, **Camera on/off**, **Speakers on/off**, **Call**, and **Devices**, each with an icon and label. Entering a private area opens a fixed call strip at the top center of the map. Collapse it in place; leaving the area hides it, and reentry opens it again. Opening the strip does not enable capture.

Settings include native microphone/camera/output selection, a local mic level test, a local camera preview, mirror preview, volume and a speaker test. Local tests do not transmit to the server and stop when settings close. Device choices are saved only when you press **Save device settings**. Capture switches start off on every launch. A conversation boundary pauses sharing and clears playout buffers; turn mic/camera on again when ready in the new conversation.

This is real native capture/playback using CPAL and native AVFoundation on macOS (Nokhwa on other supported native targets), relayed through the same server, with mono 16 kHz speech and JPEG video with the quality profiles listed above. Headphones are recommended; software echo cancellation is not implemented. It is intended for reachable, trusted networks, using the prototype's existing unencrypted transport. It is not a hosted WebRTC/SFU service.

The macOS package is **dist/Commons.app**. It includes camera, microphone and local-network usage descriptions, has camera/audio-input entitlements, and is locally ad-hoc signed and verified. **tools/package_macos.py** packages the built binary. Launch the app bundle for normal use so macOS identifies permissions as Commons. Camera access is requested only when you enable camera access, capture, or a local test. The native AVFoundation backend detected this Mac's Logi Webcam C920e and received a valid 320×240 JPEG during an opt-in self-test; no image was saved or transmitted. Microphone hardware capture still requires user testing. Audio/video codecs, playback buffering, and real TCP routing are tested.

All participants should use this updated build (wire protocol 5). Older servers are reported with an update message. For a second local connection, another app instance can join localhost; your first instance is already connected after Create server.

Additional screenshot views: **--view devices**, **--view spaces**, and **--view doors**. Tests and screenshot runs do not write user profiles or world designs. Existing user data hashes were checked before and after verification.


## Camera, layout, and appearance update

macOS camera discovery now uses AVFoundation directly with USB-driver compatibility and stable device IDs. Settings show permission status, a retry/access button, and a link to Camera privacy settings. Select the Logi webcam under **Devices**, then **Preview camera locally**. USB unplug/reconnect failures remain visible with recovery guidance.

Use **Hide** on either sidebar or **Profile → View** to control panels and the device toolbar. **Profile → View → Map only** or **F10** leaves the map, compact overlays, and one restore button. **Esc** restores controls. An unfinished room draft is suspended while you navigate and restored afterward. Active sharing remains indicated with a stop button in map-only mode.

**Profile → Avatar & profile** includes masculine and feminine bodies, matching expressions and garments, ten hairstyles, six tops per body, trousers/shorts/skirts, seven accessory choices and existing independent palette colors. All 49 character layers retain LPC author/license credits; older saved profiles remain compatible. Updated peers use protocol 5; restart older servers with this version.

**Profile → Theme** offers Light, Dark, or System and Lavender, Ocean, Forest, Rose, or Amber accents. Appearance and panel preferences persist locally; map and sprite colors stay original. Screenshot/test runs do not modify your office or profile.

Additional QA views: `--view female|dark|map|theme|devices`. `--camera-diagnostics` lists device names and permission status without capture. `--camera-self-test` only runs when camera permission is already granted, verifies one JPEG in memory, stops capture, and exits without saving or transmitting the image.


## Browser guests and hosting

**Profile → Create server** has **Let guests join in their browser** (on by default) and a browser port, default 8080. **Profile → Server & invite** offers **Copy browser link** and **Open browser**. Guests open the link, enter a name/password, and use the same Rust/egui app compiled to real WebAssembly. Browser guests can move, chat, customize characters, use doors/private areas, change themes/panels, and use browser camera/mic settings. Hosting remains a native-server function. Web participants can unlock administration with the separate administrator password; the server rejects unauthorized map changes. The WebSocket handshake never grants editor permission by itself.

One native server serves all HTML/CSS/JavaScript/WASM and bridges WebSocket guests to the existing room server. App assets are embedded and compressed; no separate dev server/CDN is needed. Desktop guests can join either a traditional TCP address or the same HTTP/HTTPS browser invite. Native WSS connections validate public TLS certificates. Browser session buffers are bounded, state updates coalesce, and privacy boundary events discard queued media.

For windowless hosting use `--serve-web` with optional `--http-bind`, `--tcp-bind`, and `--name`; set `COMMONS_PASSWORD` and `COMMONS_DATA_DIR` in the environment. Detailed local/HTTPS hosting instructions and Docker/Caddy configuration are in `deploy/README.md`. Public hosting still needs the person's domain/server destination; no internet deployment is assumed. Browser mic/camera work on HTTPS or localhost; LAN HTTP gives an explicit secure-link explanation while maps/chat continue to work.

Build the browser package with `python3 tools/build_web.py`, then `cargo build` to embed it. The browser page and shared Rust UI are actual deliverables in `web/dist`, not a screenshot/mockup. Browser tests use timed server processes and Chrome synthetic camera/mic devices; they do not record the person's webcam. Browser settings persist per origin in localStorage; desktop data stays in its existing local files. A separate `COMMONS_DATA_DIR` can be used on another machine.


## Smoother movement and browser performance

Walking pace is now 5.75 tiles/second (15% faster) for keyboard and click-to-walk. Simulation consumes elapsed time consistently across ordinary frame-rate dips, with small collision-safe steps. Click-to-walk consumes its remaining movement budget across waypoints. Remote avatars interpolate short updates and snap directly when the display is too slow, a room changes, or a position teleports.

Navigation renders each room's floor as a cached nearest-neighbor texture rather than rebuilding thousands of image shapes every frame. Floor caches invalidate on floor/size changes; live editor painting remains immediate. The browser requests an opaque graphics surface with multisampling, depth, stencil, and dithering disabled; its pixel art and egui edge rendering do not need those extra GPU buffers/effects.

Room maps transfer on initial join and after a world revision. Movement packets carry people and filtered chat instead of resending unchanged maps. State coalescing preserves pending map revisions, and native transports hydrate the omitted world from their revision cache. Movement sends/broadcasts at up to 25 Hz. Protocol 5 requires an updated host/client: restart the host and refresh the browser page after updating.

Controlled local traffic measurements (two guest connections, 1440×900, same office) reduced average movement state size from roughly 21,105 to 685 bytes, about 97% smaller; no maps were resent during the updated walking sample. Frame timing varied widely under Chrome's software graphics renderer, so these measurements do not establish a hardware-independent FPS improvement. The person confirmed Chrome or Edge; their graphics hardware and frame rate remain unverified. The tests cover walking speed at 12–120 FPS, collision safety, cached floor pixels/invalidation, world synchronization/coalescing, and browser/desktop privacy and media behavior.


## Compact workspace layout

The thin header shows the Commons mark and space title. Your name, character icon, status, avatar editor, View, Theme, and server actions live in the bottom-left profile menu. Browser guests see join/invite actions; creating a server remains a native function. Room editing is beside the ROOMS heading.

The map uses the cleared central area. Compact minus/plus buttons, a slider, percentage, and Fit float at the lower-right corner; Alt-wheel still zooms. Private calls stay fixed at the top center rather than opening a movable window. Device controls remain in the bottom dock.

Additional screenshot views: `--view profile-menu` and `--view private`. The layout checks cover actual native pointer interaction and two Chrome WebAssembly guests with synthetic devices.
