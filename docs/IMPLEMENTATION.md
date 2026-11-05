# Meetings, administration and dynamic spaces

Baseline: 09a2b435, Commons 0.4.3. Work branch: feat/meetings-admin-spaces.

1. Server-authorized web administration using a separate password; dynamic maps with stable IDs, safe deletion, world revision synchronization and server-side persistence. Preserve existing worlds; one room for a new installation.
2. Map / Meeting views, individual pinning of camera or screen, participant pagination, subscriptions to visible video sources. Map position and conversation remain unchanged when switching views.
3. Independent camera and screen capture. Multiple presenters; local selection, automatic initial screen focus without stealing an existing pin. Stop capture on conversation changes. Camera quality profiles Low/Medium/High and a readable screen profile.
4. Build native and web clients, protocol integration tests and browser tests, including authorization, persistence, concurrent streams, pinning, and privacy boundaries. Validate in an isolated deployment before updating an existing service.

Protocol and client/server releases must move together. Desktop capture uses OS permission and a source picker; web capture uses the browser picker. Physical device and human screen-picker validation remain explicit acceptance checks. The features are independent of external workflow systems.

## Implementation and checks

The four implementation stages are present on the feature branch. The final local run passes 49 Rust tests, native and WebAssembly Clippy with warnings denied, and Chrome scenarios with eight participants. A macOS application bundle is locally signed and verified. Browser capture uses synthetic devices for automated checks; no personal desktop or camera is recorded by these tests.

The preview keeps JPEG/PCM over the existing transport. Camera profiles are 320×240/6 fps, 640×480/15 fps and 1280×720/24 fps. Independent screen capture is up to 1920×1080/5 fps with no system audio. The world supports 1–128 maps, with per-map limits preserved; existing world data is normalized without deleting maps. Native Windows/Linux desktop capture remains a platform acceptance item.

## Reproducing browser validation

Build the web client with `python3 tools/build_web.py`, then run `cargo build --bin commons-desktop` to embed it. With Node.js 22 or newer, install dependencies using `npm ci --prefix tools/browser-tests` and install Chromium with `cd tools/browser-tests && npx playwright install chromium`. Run `node tools/browser-tests/features.cjs` from the repository root.

The test starts a temporary server with synthetic media and disposable credentials. It checks administrator authorization, map creation/editing/deletion, simultaneous webcam/screen streams, two presenters, pin selection, quality profiles, pagination with eight participants, and stopping screen sharing independently of the webcam. Test images and reports remain in ignored `shots/` and `data/` folders. Set `COMMONS_TEST_BROWSER` to use another Chromium executable. On macOS, an installed Google Chrome is used when available.

For a dedicated remote test instance, set `COMMONS_TEST_URL`, `COMMONS_TEST_WS_URL`, `COMMONS_TEST_PASSWORD` and `COMMONS_TEST_ADMIN_PASSWORD`. This mode creates and deletes a temporary map and uses synthetic participants. The browser test permits private HTTPS certificates; independently verify the deployment certificate before relying on that test result.
