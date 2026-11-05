use crate::*;
impl Commons {
    pub fn can_edit_world(&self) -> bool {
        (!self.connected && !cfg!(target_arch = "wasm32")) || self.is_admin || self.server.is_some()
    }
    pub fn administration_dialog(&mut self, ctx: &Context) {
        let mut open = true;
        Window::new("Administration").open(&mut open).collapsible(false).resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO).default_width(460.0).show(ctx, |ui| {
            if !self.is_admin {
                ui.label("Enter the administrator password for this workspace.");
                let r = ui.add(TextEdit::singleline(&mut self.admin_password).password(true).hint_text("Administrator password"));
                self.target("admin_password", r.rect);
                let login = ui.button("Unlock administration"); self.target("admin_login", login.rect);
                if login.clicked() {
                    let password = std::mem::take(&mut self.admin_password);
                    self.send(Command::AdminLogin { password });
                }
            } else {
                ui.label("Administrator access is active for this connection.");
                let logout = ui.button("Lock administration"); self.target("admin_logout", logout.rect);
                if logout.clicked() { self.send(Command::AdminLogout); }
                ui.separator();
                ui.label("Create a map");
                let r = ui.add(TextEdit::singleline(&mut self.new_room_name).char_limit(40).hint_text("Map name"));
                self.target("new_room_name", r.rect);
                let create = ui.add_enabled(!self.new_room_name.trim().is_empty(), Button::new("Create map"));
                self.target("room_create", create.rect);
                if create.clicked() {
                    let name = std::mem::take(&mut self.new_room_name);
                    self.send(Command::CreateRoom { name, revision: self.world_revision });
                }
                ui.separator();
                ui.label(format!("Current map: {}", self.maps[self.room].name));
                if self.maps.len() > 1 {
                    let remove = ui.button("Delete current map…"); self.target("room_delete", remove.rect);
                    if remove.clicked() { self.delete_room_id = Some(self.maps[self.room].id); }
                } else { ui.label("Keep one entrance map. Create another before deleting this one."); }
                if let Some(id) = self.delete_room_id {
                    if let Some(map) = self.maps.iter().find(|m| m.id == id) {
                        ui.label(format!("Delete ‘{}’? Participants move to the entrance; doors to this map are removed.", map.name));
                        ui.horizontal(|ui| {
                            let confirm = ui.button("Delete map"); self.target("room_delete_confirm", confirm.rect);
                            if confirm.clicked() {
                                self.send(Command::DeleteRoom { id, revision: self.world_revision }); self.delete_room_id = None;
                            }
                            if ui.button("Cancel").clicked() { self.delete_room_id = None; }
                        });
                    } else { self.delete_room_id = None; }
                }
            }
            let close = ui.button("Done"); self.target("admin_close", close.rect);
            if close.clicked() { self.dialog = None; self.admin_password.clear(); self.delete_room_id = None; }
            if !self.notice.is_empty() { ui.separator(); ui.label(&self.notice); }
        });
        if !open {
            self.dialog = None;
            self.admin_password.clear();
            self.delete_room_id = None;
        }
    }
}
