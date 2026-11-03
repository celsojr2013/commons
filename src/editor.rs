use crate::{
    world::{catalog, preset, RoomMap},
    *,
};
#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    Paint,
    Fill,
    Erase,
    Area,
    Door,
}
pub struct Editor {
    pub map: RoomMap,
    pub original: RoomMap,
    pub brush: u16,
    pub category: String,
    pub tool: Tool,
    pub width: usize,
    pub height: usize,
    pub solid: bool,
    pub undo: Vec<RoomMap>,
    pub redo: Vec<RoomMap>,
    pub stroke: Option<RoomMap>,
    pub last: Option<(usize, usize)>,
    pub template: usize,
    pub preset_name: String,
    pub area_name: String,
    pub area_width: usize,
    pub area_height: usize,
    pub door_name: String,
    pub door_destination: usize,
    pub arrival_x: usize,
    pub arrival_y: usize,
    pub selected_area: Option<u64>,
    pub selected_door: Option<u64>,
    pub connection_error: String,
}
impl Editor {
    pub fn new(map: RoomMap) -> Self {
        Self {
            width: map.width,
            height: map.height,
            original: map.clone(),
            map,
            brush: 0,
            category: "Floors".into(),
            tool: Tool::Paint,
            solid: false,
            undo: vec![],
            redo: vec![],
            stroke: None,
            last: None,
            template: 0,
            preset_name: "My room".into(),
            area_name: "Private meeting".into(),
            area_width: 6,
            area_height: 4,
            door_name: "Door".into(),
            door_destination: 1,
            arrival_x: 12,
            arrival_y: 12,
            selected_area: None,
            selected_door: None,
            connection_error: String::new(),
        }
    }
    pub fn remember(&mut self) {
        self.undo.push(self.map.clone());
        if self.undo.len() > 50 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    pub fn undo(&mut self) {
        if let Some(m) = self.undo.pop() {
            self.redo.push(self.map.clone());
            self.map = m;
            self.width = self.map.width;
            self.height = self.map.height;
        }
    }
    pub fn redo(&mut self) {
        if let Some(m) = self.redo.pop() {
            self.undo.push(self.map.clone());
            self.map = m;
            self.width = self.map.width;
            self.height = self.map.height;
        }
    }
    pub fn apply(&mut self, x: usize, y: usize, first: bool, erase: bool) {
        if first {
            self.stroke = Some(self.map.clone());
            self.last = None;
        }
        if !erase && matches!(self.tool, Tool::Area | Tool::Door) {
            if first {
                self.place_connection(x, y);
            }
            self.last = Some((x, y));
            return;
        }
        if erase || self.tool == Tool::Erase {
            if let Some(i) = self.map.doors.iter().position(|d| d.x == x && d.y == y) {
                self.map.doors.remove(i);
                return;
            }

            self.map.erase(x, y);
        } else if self.tool == Tool::Fill {
            if first {
                self.map.fill(x, y, self.brush);
            }
        } else if self.brush < 14 || catalog()[self.brush as usize].category == "Walls" {
            let previous = self.last.unwrap_or((x, y));
            let n = x.abs_diff(previous.0).max(y.abs_diff(previous.1));
            for i in 0..=n {
                let t = if n == 0 { 0.0 } else { i as f32 / n as f32 };
                let px = (previous.0 as f32 + (x as f32 - previous.0 as f32) * t).round() as usize;
                let py = (previous.1 as f32 + (y as f32 - previous.1 as f32) * t).round() as usize;
                self.map
                    .stamp(self.brush, px, py, self.brush >= 14 && self.solid);
            }
        } else if first {
            self.map.stamp(self.brush, x, y, self.solid);
        }
        self.last = Some((x, y));
    }
    pub fn finish(&mut self) {
        if let Some(before) = self.stroke.take() {
            if before != self.map {
                self.undo.push(before);
                if self.undo.len() > 50 {
                    self.undo.remove(0);
                }
                self.redo.clear();
            }
        }
        self.last = None;
    }
}
impl Commons {
    pub fn begin_edit(&mut self) {
        if cfg!(target_arch = "wasm32") || (self.connected && self.server.is_none()) {
            self.notice =
                "The host controls room designs. You can still customize your avatar.".into();
            return;
        }
        self.editor = Some(Editor::new(self.maps[self.room].clone()));
        self.people_tab = false;
        self.path.clear();
        self.walking = false;
        self.camera = Vec2::ZERO;
        self.zoom = 1.0;
    }
    pub fn save_room(&mut self) {
        let Some(editor) = &self.editor else {
            return;
        };
        if !editor.map.validate() {
            self.error = "Use a room name of 1–40 characters and keep props inside the map.".into();
            return;
        }
        let room = editor.map.clone();
        self.maps[self.room] = room.clone();
        if self.connected {
            self.send(Command::EditRoom {
                room: self.room,
                map: room,
            });
        }
        self.save_world();
        self.editor = None;
        self.error.clear();
        self.position = self.maps[self.room].spawn();
        self.path.clear();
        self.send_motion();
        self.notice = "Room design saved. Everyone on your server sees the same room.".into();
    }
    pub fn editor_panel(&mut self, ctx: &Context) {
        SidePanel::right("tile editor").exact_width(286.0).resizable(false).frame(Frame::new().fill(theme::palette(ctx).panel).inner_margin(18).stroke(Stroke::new(1.0,theme::palette(ctx).border))).show(ctx,|ui|{
  ui.horizontal(|ui|{ui.label(RichText::new("Room studio").size(19.0).strong());if ui.small_button("Hide").clicked(){self.settings.view.conversation=false;self.save_settings();}});
ui.label(RichText::new("Build a little world of your own.").size(11.0).color(theme::palette(ui.ctx()).muted));
  ui.horizontal(|ui|{
if primary(ui,"Save room").clicked(){
self.save_room();
}
if ui.button("Cancel").clicked(){
self.editor=None;
self.error.clear();
}
}
);
  let Some(editor)=&mut self.editor else{
return;
}
;
  ui.label(RichText::new(if editor.map!=editor.original{
"Unsaved changes"}
else{
"Draft · no changes yet"}
).size(10.0).color(theme::palette(ui.ctx()).muted));
  ScrollArea::vertical().id_salt("editor controls").show(ui,|ui|{
   ui.separator();
ui.label(RichText::new("ROOM").size(10.0).color(theme::palette(ui.ctx()).muted).strong());
ui.add(TextEdit::singleline(&mut editor.map.name).char_limit(40).desired_width(f32::INFINITY));
   ui.horizontal(|ui|{
ui.label("Size");
ui.add(DragValue::new(&mut editor.width).range(12..=64).suffix(" wide"));
ui.add(DragValue::new(&mut editor.height).range(12..=64).suffix(" high"));
}
);
   if editor.width!=editor.map.width||editor.height!=editor.map.height{
ui.label(RichText::new("Shrinking crops tiles and props. Undo restores them.").size(10.0).color(theme::palette(ui.ctx()).muted));
if ui.button("Apply size").clicked(){
editor.remember();
editor.map.resize(editor.width,editor.height);
}
}
   ui.label(RichText::new(format!("{} × {} tiles · {} props",editor.map.width,editor.map.height,editor.map.props.len())).size(10.0).color(theme::palette(ui.ctx()).muted));
   ui.horizontal(|ui|{
if ui.add_enabled(!editor.undo.is_empty(),Button::new("Undo")).clicked(){
editor.undo();
}
if ui.add_enabled(!editor.redo.is_empty(),Button::new("Redo")).clicked(){
editor.redo();
}
ui.checkbox(&mut self.show_grid,"Grid");
}
);
   ui.separator();
ui.horizontal(|ui|{
ui.selectable_value(&mut editor.tool,Tool::Paint,"Paint");
ui.add_enabled_ui(editor.brush<14,|ui|{
ui.selectable_value(&mut editor.tool,Tool::Fill,"Fill");
}
);
ui.selectable_value(&mut editor.tool,Tool::Erase,"Erase");
}
);
   ui.label(RichText::new("Drag to paint floors. Click to place a prop. Right-click erases props.").size(10.0).color(theme::palette(ui.ctx()).muted));
   let previous_category=editor.category.clone();
   ComboBox::from_id_salt("category").selected_text(&editor.category).width(228.0).show_ui(ui,|ui|{
for c in ["Floors","Walls","Furniture","Nature","Halloween","Areas & doors"]{
ui.selectable_value(&mut editor.category,c.into(),c);
}
}
);
   if editor.category=="Areas & doors" {if previous_category!=editor.category{editor.tool=Tool::Area;}crate::space_editor::connection_controls(ui,editor,&self.maps,self.room);}
   let assets:Vec<_>=catalog().iter().filter(|a|a.category==editor.category).collect();
   for row in assets.chunks(3){
ui.horizontal(|ui|{
for a in row{
let (r,response)=ui.allocate_exact_size(vec2(74.0,76.0),Sense::click());
let selected=editor.brush==a.id;
ui.painter().rect_filled(r,8,if selected{
theme::palette(ui.ctx()).soft}
else{
theme::palette(ui.ctx()).raised}
);
if selected{
ui.painter().rect_stroke(r,8,Stroke::new(1.5,theme::palette(ui.ctx()).accent),StrokeKind::Inside);
}
let max=vec2(44.0,43.0);
let size=vec2(a.width as f32,a.height as f32);
let size=size*(max.x/size.x).min(max.y/size.y);
self.sprites.tile(ui.painter(),a.id,Rect::from_center_size(r.center_top()+vec2(0.0,26.0),size));
ui.painter().text(r.center_bottom()-vec2(0.0,12.0),Align2::CENTER_CENTER,&a.name,FontId::proportional(9.0),if selected{
theme::palette(ui.ctx()).accent}
else{
theme::palette(ui.ctx()).text}
);
if response.on_hover_text(format!("{} · {}×{} pixels",a.name,a.width,a.height)).clicked(){
editor.brush=a.id;
editor.solid=a.solid;
editor.tool=Tool::Paint;
}
}
}
);
}
   ui.add_space(4.0);
if editor.brush>=14 && editor.category!="Areas & doors"{
ui.checkbox(&mut editor.solid,"Blocks walking");
}
else if editor.tool==Tool::Fill{
ui.label(RichText::new("Fill replaces connected floor tiles.").size(10.0).color(theme::palette(ui.ctx()).muted));
}
   ui.separator();
ui.label(RichText::new("START FROM A THEME").color(theme::palette(ui.ctx()).muted).size(10.0).strong());
ComboBox::from_id_salt("template").selected_text(["Halloween office","Cozy office","Forest garden","Blank canvas"][editor.template]).show_ui(ui,|ui|{
for(i,n)in["Halloween office","Cozy office","Forest garden","Blank canvas"].iter().enumerate(){
ui.selectable_value(&mut editor.template,i,*n);
}
}
);
   if ui.button("Apply theme").on_hover_text("Replaces the draft. Undo brings your previous design back.").clicked(){
editor.remember();
editor.map=preset(editor.template,&editor.map.name,editor.width,editor.height);
}
   ui.separator();
ui.label(RichText::new("MY SAVED DESIGNS").size(10.0).color(theme::palette(ui.ctx()).muted).strong());
ui.add(TextEdit::singleline(&mut editor.preset_name).hint_text("Design name").char_limit(32));
if ui.button("Save design to library").clicked() && !editor.preset_name.trim().is_empty()&&editor.map.validate(){
let name=editor.preset_name.trim().to_owned();
let mut saved=load_designs();
saved.retain(|(n,_)|*n!=name);
saved.push((name,editor.map.clone()));
let _=write_json(&data_dir().join("designs.json"),&saved);
self.notice="Design saved in your library.".into();
}
   for(name,map)in load_designs(){
ui.horizontal(|ui|{
ui.label(&name);
if ui.small_button("Load").clicked(){
editor.remember();
editor.map=map;
editor.width=editor.map.width;
editor.height=editor.map.height;
}
}
);
}
  }
);
 }
);
    }
}
fn load_designs() -> Vec<(String, RoomMap)> {
    storage::read(&data_dir().join("designs.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<(String, RoomMap)>>(&b).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, m)| m.validate())
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strokes_undo_redo_and_resize() {
        let map = world::preset(0, "Draft", 26, 18);
        let mut e = Editor::new(map.clone());
        e.brush = 2;
        e.apply(4, 4, true, false);
        e.apply(10, 4, false, false);
        e.finish();
        assert_eq!(e.undo.len(), 1);
        assert!((4..=10).all(|x| e.map.ground[4 * 26 + x] == 2));
        let painted = e.map.clone();
        e.undo();
        assert_eq!(e.map, map);
        e.redo();
        assert_eq!(e.map, painted);
        e.remember();
        e.map.resize(12, 12);
        assert!(e.map.validate());
        e.undo();
        assert_eq!(e.map, painted);
        e.brush = 14;
        e.solid = true;
        e.apply(2, 2, true, false);
        e.apply(6, 2, false, false);
        e.finish();
        assert!((2..=6).all(|x| !e.map.walkable(x, 2)));
    }
}
