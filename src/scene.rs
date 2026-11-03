use crate::*;
impl Commons {
    fn tick_movement(&mut self, ctx: &Context) {
        if self.editor.is_some() || self.dialog.is_some() || self.chrome.profile_open {
            if self.walking {
                self.walking = false;
                self.send_motion();
            }
            return;
        }
        let map = &self.maps[self.room];
        let mut key = Vec2::ZERO;
        let dt = ctx.input(|i| i.unstable_dt.min(0.1));
        if !ctx.wants_keyboard_input() {
            key = ctx.input(|i| {
                vec2(
                    (i.key_down(Key::ArrowRight) || i.key_down(Key::D)) as u8 as f32
                        - (i.key_down(Key::ArrowLeft) || i.key_down(Key::A)) as u8 as f32,
                    (i.key_down(Key::ArrowDown) || i.key_down(Key::S)) as u8 as f32
                        - (i.key_down(Key::ArrowUp) || i.key_down(Key::W)) as u8 as f32,
                )
            });
        }
        let old = self.position;
        let was = self.walking;
        if key != Vec2::ZERO {
            self.path.clear();
            movement::keyboard(map, &mut self.position, key, dt);
        } else {
            movement::path(map, &mut self.position, &mut self.path, dt);
        }
        self.walking = self.position != old;
        let dx = self.position[0] - old[0];
        let dy = self.position[1] - old[1];
        if self.walking {
            self.direction = if dx.abs() > dy.abs() {
                if dx > 0.0 {
                    3
                } else {
                    1
                }
            } else if dy > 0.0 {
                2
            } else {
                0
            };
        }
        if (self.walking && self.last_move.elapsed() > Duration::from_millis(40))
            || was != self.walking
        {
            self.send_motion();
            self.last_move = Instant::now();
        }
    }
    pub fn main_panel(&mut self, ctx: &Context) {
        self.tick_movement(ctx);
        self.try_door();
        self.sync_conversation();
        CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(theme::palette(ctx).background)
                    .inner_margin(if self.settings.view.map_only {0} else {8}),
            )
            .show(ctx, |ui| {
                if self.people_tab && self.editor.is_none() {
                    let rect=ui.available_rect_before_wrap();self.chrome.map_rect=Some(rect);if self.dialog.is_none(){self.call_strip(ctx,rect);}
                    self.people_view(ui);
                    return;
                }
                let map = self
                    .editor
                    .as_ref()
                    .map(|e| e.map.clone())
                    .unwrap_or_else(|| self.maps[self.room].clone());
                let (rect, response) = ui.allocate_exact_size(
                    vec2(
                        ui.available_width(),
                        ui.available_height().max(240.0),
                    ),
                    Sense::click_and_drag(),
                );
                self.chrome.map_rect=Some(rect);self.target("map",rect);if self.dialog.is_none(){self.map_controls(ctx,rect);self.call_strip(ctx,rect);self.map_toast(ctx,rect);}
                let over_overlay=self.overlay_contains(ctx);
                let p = ui.painter_at(rect);
                p.rect_filled(rect, 14, Color32::from_rgb(48, 45, 62));
                let panning = ctx.input(|i| i.key_down(Key::Space)) && !ctx.wants_keyboard_input();
                let dragging = !over_overlay && (response.dragged_by(PointerButton::Middle)
                    || (panning && response.dragged_by(PointerButton::Primary)));
                let scrolling = response.hovered() && !over_overlay
                    && ctx.input(|i| i.raw_scroll_delta != Vec2::ZERO);
                if dragging {
                    self.camera += ctx.input(|i| i.pointer.delta());
                }
                if response.hovered() && !over_overlay {
                    let alt = ctx.input(|i| i.modifiers.alt || i.events.iter().any(|e|matches!(e,egui::Event::MouseWheel{modifiers,..} if modifiers.alt)));
                    if alt {
                        let wheel = ctx.input(|i| i.raw_scroll_delta.y);
                        if wheel != 0.0 {
                            let base = ((rect.width() - 36.0) / map.width as f32)
                                .min((rect.height() - 40.0) / map.height as f32);
                            zoom::zoom_at(
                                &mut self.zoom,
                                &mut self.camera,
                                rect.center(),
                                response.hover_pos().unwrap_or(rect.center()),
                                base,
                                wheel,
                            );
                        }
                    } else {
                        self.camera += ctx.input(|i| i.raw_scroll_delta);
                    }
                }
                let cell = (((rect.width() - 36.0) / map.width as f32)
                    .min((rect.height() - 40.0) / map.height as f32)
                    * self.zoom)
                    .floor()
                    .max(4.0);
                let world_size = vec2(map.width as f32 * cell, map.height as f32 * cell);
                if self.walking && self.editor.is_none() && !dragging && !scrolling {
                    follow::track(&mut self.camera, rect, world_size, self.position, cell);
                }
                let world = Rect::from_center_size(
                    rect.center() + self.camera,
                    world_size,
                );
                self.chrome.world_rect=Some(world);
                if self.editor.is_none(){
                    let texture=self.sprites.floor_texture(ctx,self.room,&map);p.image(texture,world,Rect::from_min_max(Pos2::ZERO,pos2(1.0,1.0)),Color32::WHITE);
                }else{
                for y in 0..map.height {
                    for x in 0..map.width {
                        let tile = Rect::from_min_size(
                            world.min + vec2(x as f32 * cell, y as f32 * cell),
                            Vec2::splat(cell),
                        );
                        if tile.intersects(rect) {
                            self.sprites.tile(&p, map.ground[y * map.width + x], tile);
                        }
                    }
                }
                }
                for area in &map.areas {
                    let r = Rect::from_min_size(
                        world.min + vec2(area.x as f32 * cell, area.y as f32 * cell),
                        vec2(area.width as f32 * cell, area.height as f32 * cell),
                    );
                    p.rect_filled(r, 2, Color32::from_rgba_unmultiplied(128, 100, 195, 27));
                    p.rect_stroke(
                        r,
                        2,
                        Stroke::new(1.5, Color32::from_rgb(181, 156, 230)),
                        StrokeKind::Inside,
                    );
                    let label = format!("PRIVATE · {}", area.name);
                    let galley = p.layout_no_wrap(label, FontId::proportional(9.0), theme::palette(ctx).primary_text);
                    let tag = Rect::from_min_size(
                        r.min + vec2(2.0, 2.0),
                        galley.size() + vec2(10.0, 6.0),
                    );
                    p.rect_filled(tag, 3, theme::palette(ctx).accent);
                    p.galley(tag.min + vec2(5.0, 3.0), galley, theme::palette(ctx).primary_text);
                }
                for door in &map.doors {
                    let r = Rect::from_min_size(
                        world.min + vec2(door.x as f32 * cell, door.y as f32 * cell),
                        Vec2::splat(cell),
                    );
                    crate::space_editor::draw_door(&p, r);
                    p.text(
                        r.center_bottom() + vec2(0.0, 7.0),
                        Align2::CENTER_TOP,
                        &door.name,
                        FontId::proportional(9.0),
                        Color32::WHITE,
                    );
                }
                let mut actors: Vec<Person> = self
                    .people()
                    .into_iter()
                    .filter(|p| p.room == self.room && p.id != self.id.unwrap_or(0))
                    .collect();
                actors.push(Person {
                    id: self.id.unwrap_or(0),
                    name: format!("{} · you", self.settings.name),
                    room: self.room,
                    x: self.position[0],
                    y: self.position[1],
                    color: self.settings.color,
                    away: self.away,
                    avatar: self.settings.avatar.clone(),
                    direction: self.direction,
                    walking: self.walking,
                    mic: self.media.mic_on,
                    camera: self.media.camera_on,
                });
                let mut draw: Vec<(f32, bool, usize)> = map
                    .props
                    .iter()
                    .enumerate()
                    .map(|(i, o)| {
                        let a = &world::catalog()[o.tile as usize];
                        (o.y as f32 + a.height as f32 / 16.0, true, i)
                    })
                    .chain(
                        actors
                            .iter()
                            .enumerate()
                            .map(|(i, a)| (a.y * map.height as f32, false, i)),
                    )
                    .collect();
                draw.sort_by(|a, b| a.0.total_cmp(&b.0));
                let time = ctx.input(|i| i.time);
                for (_, prop, i) in draw {
                    if prop {
                        let object = &map.props[i];
                        let a = &world::catalog()[object.tile as usize];
                        let r = Rect::from_min_size(
                            world.min + vec2(object.x as f32 * cell, object.y as f32 * cell),
                            vec2(a.width as f32 / 16.0 * cell, a.height as f32 / 16.0 * cell),
                        );
                        if r.intersects(rect) {
                            self.sprites.tile(&p, object.tile, r);
                        }
                    } else {
                        let a = &actors[i];
                        let position=if a.id==self.id.unwrap_or(0){[a.x,a.y]}else{self.remote_motion.sample(a.id,a.room,[a.x,a.y],time)};
                        let feet = world.min + vec2(position[0] * world.width(), position[1] * world.height());
                        p.rect_filled(
                            Rect::from_center_size(feet, vec2(cell * 0.65, cell * 0.22)),
                            cell * 0.15,
                            Color32::from_black_alpha(40),
                        );
                        self.sprites.draw_avatar(
                            &p,
                            feet,
                            cell * 2.7,
                            &a.avatar,
                            (a.direction, a.walking),
                            time,
                        );
                        let name = p.layout_no_wrap(
                            a.name.clone(),
                            FontId::proportional(10.0),
                            Color32::WHITE,
                        );
                        let r = Rect::from_center_size(
                            feet + vec2(0.0, cell * 0.36),
                            name.size() + vec2(10.0, 6.0),
                        );
                        p.rect_filled(r, 4, Color32::from_rgb(43, 42, 56));
                        p.galley(r.min + vec2(5.0, 3.0), name, Color32::WHITE);
                        if a.away {
                            p.circle_filled(
                                feet + vec2(cell * 0.4, -cell),
                                4.0,
                                Color32::from_rgb(240, 189, 104),
                            );
                        }
                    }
                }
                if self.editor.is_some() && self.show_grid {
                    for x in 0..=map.width {
                        let x = world.left() + x as f32 * cell;
                        p.line_segment(
                            [pos2(x, world.top()), pos2(x, world.bottom())],
                            Stroke::new(0.5, Color32::from_white_alpha(35)),
                        );
                    }
                    for y in 0..=map.height {
                        let y = world.top() + y as f32 * cell;
                        p.line_segment(
                            [pos2(world.left(), y), pos2(world.right(), y)],
                            Stroke::new(0.5, Color32::from_white_alpha(35)),
                        );
                    }
                }
                if let Some(point) = response
                    .hover_pos()
                    .filter(|point| world.contains(*point) && !panning && !over_overlay)
                {
                    let x = ((point.x - world.left()) / cell) as usize;
                    let y = ((point.y - world.top()) / cell) as usize;
                    if let Some(editor) = &mut self.editor {
                        let hover = Rect::from_min_size(
                            world.min + vec2(x as f32 * cell, y as f32 * cell),
                            Vec2::splat(cell),
                        );
                        p.rect_stroke(
                            hover,
                            0,
                            Stroke::new(2.0, Color32::WHITE),
                            StrokeKind::Inside,
                        );
                        let down =
                            ctx.input(|i| i.pointer.primary_down() || i.pointer.secondary_down());
                        let first = ctx.input(|i| {
                            i.pointer.primary_pressed() || i.pointer.secondary_pressed()
                        });
                        if down && (first || editor.stroke.is_some()) {
                            let erase = ctx.input(|i| i.pointer.secondary_down());
                            editor.apply(x, y, first, erase);
                        }
                    } else if response.clicked() {
                        self.path = map.path(
                            self.position,
                            [
                                (x as f32 + 0.5) / map.width as f32,
                                (y as f32 + 0.5) / map.height as f32,
                            ],
                        );
                        if self.path.is_empty() {
                            self.notice = "Choose a clear floor tile to walk there.".into();
                        } else {
                            self.notice.clear();
                        }
                    }
                }
                if let Some(editor) = &mut self.editor {
                    if !ctx.input(|i| i.pointer.primary_down() || i.pointer.secondary_down()) {
                        editor.finish();
                    }
                    if !ctx.wants_keyboard_input()
                        && ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Z))
                    {
                        if ctx.input(|i| i.modifiers.shift) {
                            editor.redo();
                        } else {
                            editor.undo();
                        }
                    }
                }

            });
    }
    pub fn people_view(&mut self, ui: &mut Ui) {
        ui.add(TextEdit::singleline(&mut self.search).hint_text("Find someone…"));
        let search = self.search.to_lowercase();
        for person in self
            .people()
            .into_iter()
            .filter(|p| p.name.to_lowercase().contains(&search))
        {
            Frame::new()
                .fill(theme::palette(ui.ctx()).panel)
                .corner_radius(12)
                .inner_margin(14)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(65.0, 70.0), Sense::hover());
                        self.sprites.draw_avatar(
                            ui.painter(),
                            r.center_bottom() - vec2(0.0, 5.0),
                            85.0,
                            &person.avatar,
                            (2, false),
                            ui.input(|i| i.time),
                        );
                        ui.vertical(|ui| {
                            ui.label(RichText::new(&person.name).strong());
                            ui.label(
                                RichText::new(format!(
                                    "{} · {}",
                                    self.maps[person.room].name,
                                    if person.away { "Away" } else { "Available" }
                                ))
                                .size(12.0)
                                .color(theme::palette(ui.ctx()).muted),
                            );
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("Go to room >").clicked() {
                                self.go_room(person.room);
                                self.people_tab = false;
                            }
                        });
                    });
                });
            ui.add_space(8.0);
        }
    }
}
