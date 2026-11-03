use crate::*;
impl Commons {
    pub fn create_host(&self, bind: &str) -> Result<Server, String> {
        if cfg!(target_arch = "wasm32") {
            return Err("Browser visitors join as guests. Host from the desktop app.".into());
        }
        let mut server = Server::start_with_rooms(
            bind,
            self.settings.space.trim().to_owned(),
            self.password.clone(),
            self.maps.clone(),
        )?;
        if self.settings.browser_host {
            let port = self
                .settings
                .web_port
                .parse::<u16>()
                .map_err(|_| "Choose a browser port between 1 and 65535.")?;
            if port == 0 {
                return Err("Choose a browser port between 1 and 65535.".into());
            }
            server.enable_web(&format!("0.0.0.0:{port}"))?;
        }
        Ok(server)
    }
    pub fn host_options(&mut self, ui: &mut Ui) {
        ui.checkbox(
            &mut self.settings.browser_host,
            "Let guests join in their browser",
        );
        if self.settings.browser_host {
            field(ui, "Browser port", &mut self.settings.web_port);
            ui.label(RichText::new("Guests open the browser link in their invite. Camera and mic require HTTPS outside localhost.").size(11.0).color(theme::palette(ui.ctx()).muted));
        }
    }
    pub fn browser_invite(&mut self, ui: &mut Ui) {
        if let Some(port) = self.server.as_ref().and_then(|s| s.web_port()) {
            let link = format!("http://{}:{port}", network::local_address());
            ui.label("Join in a browser · no installation");
            ui.label(
                RichText::new(&link)
                    .monospace()
                    .color(theme::palette(ui.ctx()).accent),
            );
            ui.horizontal(|ui| {
                if primary(ui, "Copy browser link").clicked() {
                    ui.ctx().copy_text(link.clone());
                }
                if ui.button("Open browser").clicked() {
                    open_url(&format!("http://localhost:{port}"));
                }
            });
            ui.separator();
        }
    }
}
pub fn open_url(url: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = web_sys::window().and_then(|w| w.open_with_url_and_target(url, "_blank").ok());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let command = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        let _ = std::process::Command::new(command).arg(url).spawn();
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub fn serve_cli() -> Result<(), String> {
    let mut bind = "0.0.0.0:8080".to_owned();
    let mut tcp = "127.0.0.1:7777".to_owned();
    let mut name = "Our workspace".to_owned();
    let mut seconds = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--http-bind" => bind = args.next().ok_or("--http-bind needs an address")?,
            "--tcp-bind" => tcp = args.next().ok_or("--tcp-bind needs an address")?,
            "--name" => name = args.next().ok_or("--name needs a workspace name")?,
            "--run-for-seconds" => {
                seconds = Some(
                    args.next()
                        .ok_or("--run-for-seconds needs a number")?
                        .parse::<u64>()
                        .map_err(|_| "Invalid duration")?,
                )
            }
            _ => {}
        }
    }
    let password = std::env::var("COMMONS_PASSWORD").unwrap_or_default();
    let mut server = Server::start_with_rooms(&tcp, name, password, load_world())?;
    server.enable_web(&bind)?;
    println!(
        "Commons workspace is ready. Browser: http://localhost:{}  Desktop TCP: {}",
        server.web_port().unwrap(),
        server.address
    );
    if let Some(seconds) = seconds {
        std::thread::sleep(Duration::from_secs(seconds));
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?
            .block_on(tokio::signal::ctrl_c())
            .map_err(|e| e.to_string())?;
    }
    drop(server);
    Ok(())
}
