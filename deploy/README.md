# Open the workspace from a link

In the desktop app, open Commons, choose **Profile → Create server**, and leave **Let guests join in their browser** enabled. The default browser port is 8080; the desktop TCP port remains 7777. Open **Profile → Server & invite → Copy browser link** to invite guests. **Open browser** opens the local browser version on the host computer. The desktop host keeps the server running and controls the room designs. Guests only need a current browser. Hosting remains native; authorized administrators can edit maps through either client.

The browser downloads the same Rust/egui application as WebAssembly. Maps, avatars, chat, private conversations, doors, device controls, and appearance preferences work with desktop participants. The host serves all app files and WebSocket traffic together. No CDN or separate app hosting is required. The WebAssembly download is about 2 MB compressed.

## Host without a desktop window

Run the supplied native binary with `--serve-web`. Optional flags are `--http-bind 0.0.0.0:8080`, `--tcp-bind 127.0.0.1:7777`, and `--name "Our office"`. Set `COMMONS_PASSWORD` for workspace entry and a separate `COMMONS_ADMIN_PASSWORD` for administration in the process environment; passwords are not put in invite links or process arguments. `COMMONS_DATA_DIR` selects a writable settings/map folder. Place a previously saved `world.json` there to host your customized office. The server starts with one map when no valid world file exists. Press Ctrl+C to stop. The browser server and all guest connections stop with it.

Browser visitors open the HTTP/HTTPS address and enter a name and workspace password. A native app can also join that same HTTP/HTTPS invite, using WebSocket/TLS transport. Raw desktop TCP addresses remain supported for trusted networks. The native host can edit shared maps. Browser participants unlock editing through **Profile → Administration** using the administrator password; authorization is enforced by the server.

## Internet hosting with HTTPS

A public deployment needs a domain pointing to your server and incoming ports 80 and 443. Camera, microphone and screen sharing require HTTPS in browsers; `http://localhost` also works for local tests. An ordinary LAN HTTP link supports maps and chat, with an explicit message explaining how to enable browser devices.

The supplied Compose configuration runs Commons behind Caddy, which provisions and renews the domain's HTTPS certificate and forwards WebSocket connections. Copy `deploy/.env.example` to `deploy/.env`, enter your domain and separate workspace/admin passwords, then run `docker compose --env-file deploy/.env -f deploy/compose.yaml up --build -d`. Guests use `https://your-domain`. Keep the TCP port private; this deployment exposes only the HTTPS app endpoint. The named data volume preserves your world. Import a customized `world.json` into that volume before starting a replacement server.

The macOS app and browser flows have automated validation. Linux headless hosting has also been tested behind Caddy on a local network. Public-domain deployment and native Windows/Linux capture still require platform-specific validation.

## Build it again

Install the Rust `wasm32-unknown-unknown` target and the wasm-bindgen CLI matching Cargo.lock (currently 0.2.130). Run `python3 tools/build_web.py`, then `cargo build`; the native binary embeds the complete web package. On macOS, run `python3 tools/package_macos.py` to create the signed app. The build tool can use a workspace-local wasm-bindgen executable in `tools/web-toolchain/bin`.

The browser's own permissions select access to devices. Camera previews and mic tests stay local until the connected guest enables sharing. Device capture stops on page exit; crossing conversation boundaries pauses sharing and clears queued media as on desktop. Browsers that cannot choose an output device use the system output. Use headphones; the inherited low-bandwidth PCM/JPEG transport is intended for small workspaces rather than large conferences.


## Acceptance checks

Join from two real machines; enable a webcam and select a display/window for sharing. Confirm both arrive, then share from the second participant and choose/pin each source independently. Switch Map/Meeting, change camera quality, and stop screen sharing while the webcam stays on. Test administrator create/edit/delete/logout, including deleting an occupied map and reconnecting after edits. Synthetic browser tests cover these flows but do not replace physical-device and OS permission validation.

For LAN hosting, use a certificate trusted by the participating machines. The native WSS client uses public certificate roots and does not automatically import a locally installed private CA. The native client can use HTTP/WebSocket or TCP on a trusted LAN.
