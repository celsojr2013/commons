use crate::*;
use crate::{
    editor::Tool,
    world::{Door, PrivateArea},
};
impl Editor {
    pub fn place_connection(&mut self, x: usize, y: usize) {
        self.connection_error.clear();
        if x >= self.map.width || y >= self.map.height {
            return;
        }
        if self.tool == Tool::Area {
            if let Some(area) = self
                .map
                .areas
                .iter()
                .find(|a| x >= a.x && x < a.x + a.width && y >= a.y && y < a.y + a.height)
                .cloned()
            {
                self.selected_area = Some(area.id);
                self.area_name = area.name;
                self.area_width = area.width;
                self.area_height = area.height;
                return;
            }
            let area = PrivateArea {
                id: rand::random::<u64>().max(1),
                name: self.area_name.trim().into(),
                x,
                y,
                width: self.area_width,
                height: self.area_height,
            };
            let before = self.map.clone();
            self.map.areas.push(area.clone());
            if self.map.validate() {
                self.selected_area = Some(area.id);
            } else {
                self.map = before;
                self.connection_error="Choose an empty area inside the map. Private areas cannot overlap; names must be 1–32 characters.".into();
            }
        } else {
            if let Some(door) = self
                .map
                .doors
                .iter()
                .find(|d| d.x == x && d.y == y)
                .cloned()
            {
                self.selected_door = Some(door.id);
                self.door_name = door.name;
                self.door_destination = door.destination;
                self.arrival_x = door.arrival_x;
                self.arrival_y = door.arrival_y;
                return;
            }
            if !self.map.walkable(x, y) {
                self.connection_error = "Place the door on a clear floor tile.".into();
                return;
            }
            let door = Door {
                id: rand::random::<u64>().max(1),
                name: self.door_name.trim().into(),
                x,
                y,
                destination: self.door_destination,
                arrival_x: self.arrival_x,
                arrival_y: self.arrival_y,
            };
            let before = self.map.clone();
            self.map.doors.push(door.clone());
            if self.map.validate() {
                self.selected_door = Some(door.id);
            } else {
                self.map = before;
                self.connection_error =
                    "Choose a door name of 1–32 characters and a valid destination.".into();
            }
        }
    }
}
pub fn connection_controls(ui: &mut Ui, editor: &mut Editor, maps: &[RoomMap], current: usize) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut editor.tool, Tool::Area, "Private area");
        ui.selectable_value(&mut editor.tool, Tool::Door, "Door");
    });
    if editor.tool == Tool::Area {
        ui.label(
            RichText::new(
                "Click to place a private rectangle, or select an existing one on the map.",
            )
            .size(11.0)
            .color(theme::palette(ui.ctx()).muted),
        );
        ui.add(
            TextEdit::singleline(&mut editor.area_name)
                .char_limit(32)
                .hint_text("Area name"),
        );
        ui.horizontal(|ui| {
            ui.add(
                DragValue::new(&mut editor.area_width)
                    .range(2..=editor.map.width)
                    .suffix(" wide"),
            );
            ui.add(
                DragValue::new(&mut editor.area_height)
                    .range(2..=editor.map.height)
                    .suffix(" high"),
            );
        });
        if let Some(id) = editor.selected_area {
            if let Some(area) = editor.map.areas.iter().find(|a| a.id == id).cloned() {
                ui.label(format!("Selected: {}", area.name));
                if ui.button("Update selected area").clicked() {
                    let before = editor.map.clone();
                    editor.remember();
                    if let Some(a) = editor.map.areas.iter_mut().find(|a| a.id == id) {
                        a.name = editor.area_name.trim().into();
                        a.width = editor.area_width;
                        a.height = editor.area_height;
                    }
                    if !editor.map.validate() {
                        editor.map = before;
                        editor.undo.pop();
                        editor.connection_error="Keep the rectangle inside the map without overlapping another private area.".into();
                    }
                }
                if ui.button("Remove selected area").clicked() {
                    editor.remember();
                    editor.map.areas.retain(|a| a.id != id);
                    editor.selected_area = None;
                }
            }
        }
        ui.label(RichText::new("Text, voice and video stay between people currently inside. Leaving pauses your mic and camera.").size(11.0).color(theme::palette(ui.ctx()).accent));
        let areas: Vec<_> = editor
            .map
            .areas
            .iter()
            .map(|a| (a.id, a.name.clone()))
            .collect();
        for (id, name) in areas {
            if ui
                .selectable_label(editor.selected_area == Some(id), name)
                .clicked()
            {
                let a = editor.map.areas.iter().find(|a| a.id == id).unwrap();
                editor.area_name = a.name.clone();
                editor.area_width = a.width;
                editor.area_height = a.height;
                editor.selected_area = Some(id);
            }
        }
    } else if editor.tool == Tool::Door {
        ui.label(
            RichText::new(
                "Click a clear tile to place a door. Walking onto it takes you to its destination.",
            )
            .size(11.0)
            .color(theme::palette(ui.ctx()).muted),
        );
        ui.add(
            TextEdit::singleline(&mut editor.door_name)
                .char_limit(32)
                .hint_text("Door name"),
        );
        ComboBox::from_id_salt("door destination")
            .selected_text(&maps[editor.door_destination].name)
            .width(230.0)
            .show_ui(ui, |ui| {
                for (i, map) in maps.iter().enumerate() {
                    ui.selectable_value(&mut editor.door_destination, i, &map.name);
                }
            });
        let destination = if editor.door_destination == current {
            &editor.map
        } else {
            &maps[editor.door_destination]
        };
        editor.arrival_x = editor.arrival_x.min(destination.width - 1);
        editor.arrival_y = editor.arrival_y.min(destination.height - 1);
        ui.label("Arrival tile (column / row)");
        ui.horizontal(|ui| {
            ui.add(DragValue::new(&mut editor.arrival_x).range(0..=destination.width - 1));
            ui.add(DragValue::new(&mut editor.arrival_y).range(0..=destination.height - 1));
        });
        if ui.button("Use room entrance").clicked() {
            let spawn = destination.spawn();
            editor.arrival_x = (spawn[0] * destination.width as f32) as usize;
            editor.arrival_y = (spawn[1] * destination.height as f32) as usize;
        }
        if let Some(id) = editor.selected_door {
            if editor.map.doors.iter().any(|d| d.id == id) {
                if ui.button("Update selected door").clicked() {
                    editor.remember();
                    if let Some(d) = editor.map.doors.iter_mut().find(|d| d.id == id) {
                        d.name = editor.door_name.trim().into();
                        d.destination = editor.door_destination;
                        d.arrival_x = editor.arrival_x;
                        d.arrival_y = editor.arrival_y;
                    }
                }
                if ui.button("Remove selected door").clicked() {
                    editor.remember();
                    editor.map.doors.retain(|d| d.id != id);
                    editor.selected_door = None;
                }
            }
        }
        for door in editor.map.doors.clone() {
            if ui
                .selectable_label(
                    editor.selected_door == Some(door.id),
                    format!("{} > {}", door.name, maps[door.destination].name),
                )
                .clicked()
            {
                editor.selected_door = Some(door.id);
                editor.door_name = door.name;
                editor.door_destination = door.destination;
                editor.arrival_x = door.arrival_x;
                editor.arrival_y = door.arrival_y;
            }
        }
    }
    if !editor.connection_error.is_empty() {
        ui.colored_label(Color32::from_rgb(177, 61, 61), &editor.connection_error);
    }
}
pub fn draw_door(p: &Painter, cell: Rect) {
    let scale = cell.width() / 16.0;
    let origin = cell.min - vec2(0.0, 8.0 * scale);
    let block = |x: f32, y: f32, w: f32, h: f32, color: Color32| {
        p.rect_filled(
            Rect::from_min_size(origin + vec2(x, y) * scale, vec2(w, h) * scale),
            0,
            color,
        );
    };
    block(1.0, 0.0, 14.0, 24.0, Color32::from_rgb(52, 40, 64));
    block(3.0, 2.0, 10.0, 21.0, Color32::from_rgb(162, 111, 81));
    block(4.0, 3.0, 8.0, 18.0, Color32::from_rgb(191, 140, 99));
    block(7.0, 3.0, 1.0, 18.0, Color32::from_rgb(146, 96, 78));
    block(10.0, 12.0, 2.0, 2.0, Color32::from_rgb(249, 214, 133));
    block(3.0, 1.0, 10.0, 2.0, Color32::from_rgb(176, 151, 231));
}
impl Commons {
    pub fn try_door(&mut self) {
        if self.editor.is_some() || self.dialog.is_some() {
            return;
        }
        let x = (self.position[0] * self.maps[self.room].width as f32) as usize;
        let y = (self.position[1] * self.maps[self.room].height as f32) as usize;
        let current = (self.room, x, y);
        if self.door_block == Some(current) {
            return;
        }
        self.door_block = None;
        let Some(door) = self.maps[self.room].door_at(self.position).cloned() else {
            return;
        };
        let Some(target) = self.maps.get(door.destination) else {
            return;
        };
        let position = [
            (door.arrival_x as f32 + 0.5) / target.width as f32,
            (door.arrival_y as f32 + 0.5) / target.height as f32,
        ];
        let position = if target.can_stand(position) {
            position
        } else {
            target.spawn()
        };
        self.room = door.destination;
        self.position = position;
        self.path.clear();
        self.walking = false;
        self.camera = Vec2::ZERO;
        self.zoom = 1.0;
        self.door_block = Some((
            self.room,
            (position[0] * target.width as f32) as usize,
            (position[1] * target.height as f32) as usize,
        ));
        let paused = self.media.mic_on || self.media.camera_on;
        self.send_motion();
        self.notice = format!(
            "Entered {} through {}.{}",
            self.maps[self.room].name,
            door.name,
            if paused {
                " Mic and camera paused."
            } else {
                ""
            }
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create_edit_and_undo_private_areas_and_doors() {
        let mut e = Editor::new(world::RoomMap::blank("Studio", 20, 20, 0));
        e.tool = Tool::Area;
        e.apply(2, 2, true, false);
        e.finish();
        assert_eq!(e.map.areas.len(), 1);
        assert!(e.map.validate());
        let area = e.map.areas[0].clone();
        assert_eq!(e.map.area_at([0.2, 0.2]).unwrap().id, area.id);
        assert!(e.map.area_at([0.8, 0.8]).is_none());
        e.undo();
        assert!(e.map.areas.is_empty());
        e.redo();
        assert_eq!(e.map.areas[0], area);
        e.tool = Tool::Door;
        e.apply(10, 10, true, false);
        e.finish();
        assert_eq!(e.map.doors.len(), 1);
        assert!(e.map.door_at([0.525, 0.525]).is_some());
        e.map.resize(12, 12);
        assert!(e.map.validate());
        e.map.resize(12, 12);
        assert!(e.map.validate());
        let mut bad = e.map.clone();
        bad.areas.push(area);
        assert!(!bad.validate());
    }
}
