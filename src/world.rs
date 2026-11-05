use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::OnceLock};
#[derive(Clone, Debug, Deserialize)]
pub struct Asset {
    pub id: u16,
    pub name: String,
    pub category: String,
    pub width: u32,
    pub height: u32,
    pub solid: bool,
    pub x: u32,
    pub y: u32,
}
pub fn catalog() -> &'static Vec<Asset> {
    static C: OnceLock<Vec<Asset>> = OnceLock::new();
    C.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/environment.json"))
            .expect("bundled tile catalog")
    })
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Prop {
    pub tile: u16,
    pub x: usize,
    pub y: usize,
    pub solid: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PrivateArea {
    pub id: u64,
    pub name: String,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Door {
    pub id: u64,
    pub name: String,
    pub x: usize,
    pub y: usize,
    pub destination: usize,
    pub arrival_x: usize,
    pub arrival_y: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct TalkScope {
    pub room: usize,
    pub area: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RoomMap {
    #[serde(default)]
    pub id: u64,
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub ground: Vec<u16>,
    pub props: Vec<Prop>,
    #[serde(default)]
    pub areas: Vec<PrivateArea>,
    #[serde(default)]
    pub doors: Vec<Door>,
}
impl RoomMap {
    pub fn blank(name: &str, width: usize, height: usize, tile: u16) -> Self {
        let width = width.clamp(12, 64);
        let height = height.clamp(12, 64);
        Self {
            id: 0,
            name: name.into(),
            width,
            height,
            ground: vec![tile; width * height],
            props: vec![],
            areas: vec![],
            doors: vec![],
        }
    }
    pub fn validate(&self) -> bool {
        (12..=64).contains(&self.width)
            && (12..=64).contains(&self.height)
            && !self.name.trim().is_empty()
            && self.name.chars().count() <= 40
            && self.ground.len() == self.width * self.height
            && self.ground.iter().all(|t| *t < 14)
            && self.props.len() <= 512
            && self.validate_spaces()
            && self.props.iter().all(|p| {
                let a = catalog().get(p.tile as usize);
                a.is_some_and(|a| {
                    a.category != "Floors"
                        && p.x < self.width
                        && p.y < self.height
                        && p.x + (a.width as usize).div_ceil(16) <= self.width
                        && p.y + (a.height as usize).div_ceil(16) <= self.height
                })
            })
    }
    fn validate_spaces(&self) -> bool {
        if self.areas.len() > 32 || self.doors.len() > 64 {
            return false;
        }
        let mut ids = std::collections::HashSet::new();
        for a in &self.areas {
            if a.id == 0
                || !ids.insert(a.id)
                || a.name.trim().is_empty()
                || a.name.chars().count() > 32
                || a.x >= self.width
                || a.y >= self.height
                || a.width < 2
                || a.height < 2
                || a.width > self.width - a.x
                || a.height > self.height - a.y
            {
                return false;
            }
        }
        for (i, a) in self.areas.iter().enumerate() {
            if self.areas.iter().skip(i + 1).any(|b| {
                a.x < b.x + b.width
                    && b.x < a.x + a.width
                    && a.y < b.y + b.height
                    && b.y < a.y + a.height
            }) {
                return false;
            }
        }
        let mut cells = std::collections::HashSet::new();
        for d in &self.doors {
            if d.id == 0
                || !ids.insert(d.id)
                || !cells.insert((d.x, d.y))
                || d.name.trim().is_empty()
                || d.name.chars().count() > 32
                || d.x >= self.width
                || d.y >= self.height
                || d.destination >= 4
                || d.arrival_x >= 64
                || d.arrival_y >= 64
            {
                return false;
            }
        }
        true
    }
    pub fn area_at(&self, position: [f32; 2]) -> Option<&PrivateArea> {
        let x = position[0] * self.width as f32;
        let y = position[1] * self.height as f32;
        self.areas.iter().find(|a| {
            x >= a.x as f32
                && x < (a.x + a.width) as f32
                && y >= a.y as f32
                && y < (a.y + a.height) as f32
        })
    }
    pub fn scope(&self, room: usize, position: [f32; 2]) -> TalkScope {
        TalkScope {
            room,
            area: self.area_at(position).map(|a| a.id),
        }
    }
    pub fn door_at(&self, position: [f32; 2]) -> Option<&Door> {
        let x = (position[0] * self.width as f32) as usize;
        let y = (position[1] * self.height as f32) as usize;
        self.doors.iter().find(|d| d.x == x && d.y == y)
    }
    pub fn resize(&mut self, w: usize, h: usize) {
        let w = w.clamp(12, 64);
        let h = h.clamp(12, 64);
        let mut ground = vec![self.ground[0]; w * h];
        for y in 0..h.min(self.height) {
            for x in 0..w.min(self.width) {
                ground[y * w + x] = self.ground[y * self.width + x];
            }
        }
        self.width = w;
        self.height = h;
        self.ground = ground;
        self.areas.retain_mut(|a| {
            if a.x + 2 > w || a.y + 2 > h {
                return false;
            }
            a.width = a.width.min(w - a.x);
            a.height = a.height.min(h - a.y);
            true
        });
        self.doors.retain(|d| d.x < w && d.y < h);
        self.props.retain(|p| {
            let a = &catalog()[p.tile as usize];
            p.x + (a.width as usize).div_ceil(16) <= w
                && p.y + (a.height as usize).div_ceil(16) <= h
        });
    }
    pub fn stamp(&mut self, tile: u16, x: usize, y: usize, solid: bool) {
        if x >= self.width || y >= self.height {
            return;
        }
        let Some(a) = catalog().get(tile as usize) else {
            return;
        };
        if a.category == "Floors" {
            self.ground[y * self.width + x] = tile;
        } else if self.props.len() < 512
            && x + (a.width as usize).div_ceil(16) <= self.width
            && y + (a.height as usize).div_ceil(16) <= self.height
        {
            self.props.retain(|p| p.x != x || p.y != y);
            self.props.push(Prop { tile, x, y, solid });
        }
    }
    pub fn erase(&mut self, x: usize, y: usize) {
        if let Some(i) = self.props.iter().rposition(|p| {
            let a = &catalog()[p.tile as usize];
            x >= p.x
                && x < p.x + (a.width as usize).div_ceil(16)
                && y >= p.y
                && y < p.y + (a.height as usize).div_ceil(16)
        }) {
            self.props.remove(i);
        }
    }
    pub fn fill(&mut self, x: usize, y: usize, tile: u16) {
        if tile >= 14 || x >= self.width || y >= self.height {
            return;
        }
        let old = self.ground[y * self.width + x];
        if old == tile {
            return;
        }
        let mut q = vec![(x, y)];
        while let Some((x, y)) = q.pop() {
            let i = y * self.width + x;
            if self.ground[i] != old {
                continue;
            }
            self.ground[i] = tile;
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx < self.width && ny < self.height {
                    q.push((nx, ny));
                }
            }
        }
    }
    pub fn walkable(&self, x: usize, y: usize) -> bool {
        x < self.width
            && y < self.height
            && !self.props.iter().any(|p| {
                let a = &catalog()[p.tile as usize];
                p.solid
                    && x >= p.x
                    && x < p.x + (a.width as usize).div_ceil(16)
                    && y >= p.y
                    && y < p.y + (a.height as usize).div_ceil(16)
            })
    }
    pub fn can_stand(&self, p: [f32; 2]) -> bool {
        p[0] > 0.0
            && p[1] > 0.0
            && p[0] < 1.0
            && p[1] < 1.0
            && self.walkable(
                (p[0] * self.width as f32) as usize,
                (p[1] * self.height as f32) as usize,
            )
    }
    pub fn spawn(&self) -> [f32; 2] {
        let cx = self.width / 2;
        let cy = self.height * 2 / 3;
        let cell = (0..self.width * self.height)
            .filter_map(|i| {
                let x = i % self.width;
                let y = i / self.width;
                self.walkable(x, y).then_some((x, y))
            })
            .min_by_key(|(x, y)| x.abs_diff(cx) + y.abs_diff(cy))
            .unwrap_or((cx, cy));
        [
            (cell.0 as f32 + 0.5) / self.width as f32,
            (cell.1 as f32 + 0.5) / self.height as f32,
        ]
    }
    pub fn path(&self, start: [f32; 2], end: [f32; 2]) -> VecDeque<[f32; 2]> {
        let start = ((start[0] * self.width as f32) as usize).min(self.width - 1)
            + ((start[1] * self.height as f32) as usize).min(self.height - 1) * self.width;
        let ex = ((end[0] * self.width as f32) as usize).min(self.width - 1);
        let ey = ((end[1] * self.height as f32) as usize).min(self.height - 1);
        if !self.walkable(ex, ey) {
            return VecDeque::new();
        }
        let end = ey * self.width + ex;
        let mut previous = vec![usize::MAX; self.width * self.height];
        let mut queue = VecDeque::from([start]);
        previous[start] = start;
        while let Some(i) = queue.pop_front() {
            if i == end {
                break;
            }
            let x = i % self.width;
            let y = i / self.width;
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx < self.width && ny < self.height && self.walkable(nx, ny) {
                    let n = ny * self.width + nx;
                    if previous[n] == usize::MAX {
                        previous[n] = i;
                        queue.push_back(n);
                    }
                }
            }
        }
        if previous[end] == usize::MAX {
            return VecDeque::new();
        }
        let mut route = VecDeque::new();
        let mut i = end;
        while i != start {
            route.push_front([
                ((i % self.width) as f32 + 0.5) / self.width as f32,
                ((i / self.width) as f32 + 0.5) / self.height as f32,
            ]);
            i = previous[i];
        }
        route
    }
}
pub fn preset(style: usize, name: &str, w: usize, h: usize) -> RoomMap {
    let mut m = RoomMap::blank(
        name,
        w,
        h,
        if style == 2 {
            7
        } else if style == 0 {
            1
        } else {
            0
        },
    );
    if style == 3 {
        return m;
    }
    let w = m.width;
    let h = m.height;
    if style != 2 {
        for x in 0..w {
            m.stamp(
                if x % 5 == 2 {
                    18
                } else if style == 0 {
                    16
                } else {
                    14
                },
                x,
                0,
                true,
            );
            m.stamp(if style == 0 { 16 } else { 14 }, x, h - 1, true);
        }
        for y in 1..h - 1 {
            m.stamp(if style == 0 { 16 } else { 14 }, 0, y, true);
            m.stamp(if style == 0 { 16 } else { 14 }, w - 1, y, true);
        }
        for y in 3..(h - 3).min(11) {
            for x in 3..(w - 4).min(14) {
                m.ground[y * w + x] = if style == 0 { 10 } else { 11 };
            }
        }
        for (x, y) in [(4, 4), (w / 2, 4), (4, h / 2 + 1), (w / 2, h / 2 + 1)] {
            m.stamp(38, x, y, true);
            m.stamp(21, x + 1, y + 2, true);
        }
        if w > 20 {
            m.stamp(23, w - 6, 3, true);
            m.stamp(30, w - 6, 6, true);
            m.stamp(25, w - 4, h - 4, true);
            m.stamp(31, w - 3, 2, true);
        }
        m.stamp(31, 2, 2, true);
        m.stamp(32, 2, h - 3, true);
        if style == 0 {
            if w >= 20 && h >= 14 {
                m.areas.push(PrivateArea {
                    id: 1,
                    name: "Private lounge".into(),
                    x: w - 8,
                    y: 2,
                    width: 7,
                    height: 7,
                });
            }
            for (x, y) in [(2, 1), (w - 3, 1), (2, h - 3), (w - 3, h - 3)] {
                m.stamp(45, x, y, true);
            }
            m.stamp(46, w - 5, h / 2, false);
            m.stamp(47, w / 2, 1, false);
            m.stamp(48, 1, 1, false);
            m.stamp(48, w - 2, 1, false);
            m.stamp(34, w - 5, 7, false);
        }
    } else {
        for y in 0..h {
            for x in 0..w {
                if x == w / 2 || x == w / 2 + 1 || y == h / 2 {
                    m.ground[y * w + x] = 12;
                }
            }
        }
        for (x, y) in [(2, 1), (w - 4, 1), (2, h - 4), (w - 4, h - 4)] {
            m.stamp(39, x, y, true);
        }
        m.stamp(20, w / 2 - 4, h / 2 - 3, true);
        m.stamp(43, w - 5, h / 2 + 2, false);
        m.stamp(42, 3, h / 2, false);
    }
    m
}
#[cfg(test)]
pub fn default_world() -> Vec<RoomMap> {
    let mut rooms = vec![
        preset(0, "The commons", 26, 18),
        preset(1, "Focus studio", 24, 18),
        preset(1, "Meeting room", 24, 18),
        preset(2, "Coffee garden", 26, 18),
    ];
    for i in 0..4 {
        let destination = (i + 1) % 4;
        let to = rooms[destination].spawn();
        let arrival_x = (to[0] * rooms[destination].width as f32) as usize;
        let arrival_y = (to[1] * rooms[destination].height as f32) as usize;
        let x = rooms[i].width / 2;
        let y = rooms[i].height - 2;
        let name = format!("To {}", rooms[destination].name);
        rooms[i].doors.push(Door {
            id: 100 + i as u64,
            name,
            x,
            y,
            destination,
            arrival_x,
            arrival_y,
        });
    }
    normalize_world(&mut rooms);
    rooms
}
pub fn repair_targets(rooms: &mut [RoomMap]) {
    let sizes: Vec<_> = rooms
        .iter()
        .map(|r| (r.width, r.height, r.spawn()))
        .collect();
    for room in rooms {
        room.doors.retain(|door| door.destination < sizes.len());
        for door in &mut room.doors {
            if let Some((w, h, _)) = sizes.get(door.destination) {
                door.arrival_x = door.arrival_x.min(w - 1);
                door.arrival_y = door.arrival_y.min(h - 1);
            }
        }
    }
}

/// Legacy room arrays gain stable IDs without changing their order or artwork.
pub fn normalize_world(rooms: &mut Vec<RoomMap>) {
    if rooms.is_empty() {
        rooms.push(preset(1, "The commons", 26, 18));
    }
    let mut used = std::collections::HashSet::new();
    let mut next = rooms
        .iter()
        .map(|r| r.id)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    for room in rooms.iter_mut() {
        if room.id == 0 || !used.insert(room.id) {
            while next == 0 || used.contains(&next) {
                next = next.wrapping_add(1);
            }
            room.id = next;
            used.insert(next);
            next = next.wrapping_add(1);
        }
    }
    repair_targets(rooms);
}
pub fn valid_world(rooms: &[RoomMap]) -> bool {
    !rooms.is_empty() && rooms.len() <= 128 && rooms.iter().all(RoomMap::validate)
}
pub fn initial_world() -> Vec<RoomMap> {
    let mut rooms = vec![];
    normalize_world(&mut rooms);
    rooms
}
/// Door targets are legacy array offsets; repair them when an identified map is removed.
#[cfg(any(test, not(target_arch = "wasm32")))]
pub fn remove_room(rooms: &mut Vec<RoomMap>, id: u64) -> Result<usize, String> {
    let index = rooms
        .iter()
        .position(|r| r.id == id)
        .ok_or("This map no longer exists.")?;
    if rooms.len() == 1 {
        return Err("Keep one entrance map. Create another map before deleting this one.".into());
    }
    rooms.remove(index);
    for map in rooms.iter_mut() {
        map.doors.retain(|d| d.destination != index);
        for door in &mut map.doors {
            if door.destination > index {
                door.destination -= 1;
            }
        }
    }
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_resize_and_paths() {
        for mut m in default_world() {
            assert!(m.validate());
            let start = m.spawn();
            assert!(m.can_stand(start));
            m.resize(12, 12);
            assert!(m.validate());
            m.resize(64, 64);
            assert!(m.validate());
        }
        let mut m = RoomMap::blank("Maze", 12, 12, 0);
        m.stamp(14, 5, 5, true);
        let p = m.path([0.2, 0.45], [0.8, 0.45]);
        assert!(!p.is_empty());
        assert!(p.iter().all(|p| m.can_stand(*p)));
    }
    #[test]
    fn fill_and_erase() {
        let mut m = RoomMap::blank("Room", 12, 12, 0);
        m.fill(0, 0, 2);
        assert!(m.ground.iter().all(|t| *t == 2));
        m.stamp(38, 3, 3, true);
        assert!(!m.walkable(4, 4));
        m.erase(4, 4);
        assert!(m.walkable(4, 4));
        m.ground.push(0);
        assert!(!m.validate());
    }
}

#[cfg(test)]
mod dynamic_tests {
    use super::*;
    #[test]
    fn legacy_ids_and_doors_survive_creation_and_deletion() {
        let mut rooms = default_world();
        for room in &mut rooms {
            room.id = 0;
        }
        normalize_world(&mut rooms);
        let identities: Vec<_> = rooms.iter().map(|r| r.id).collect();
        let removed = rooms[1].id;
        assert_eq!(remove_room(&mut rooms, removed).unwrap(), 1);
        assert_eq!(
            rooms.iter().map(|r| r.id).collect::<Vec<_>>(),
            vec![identities[0], identities[2], identities[3]]
        );
        assert!(rooms[0].doors.is_empty());
        assert_eq!(rooms[1].doors[0].destination, 2);
        assert!(rooms
            .iter()
            .flat_map(|r| &r.doors)
            .all(|d| d.destination < rooms.len()));
        let mut empty = vec![];
        normalize_world(&mut empty);
        assert_eq!(empty.len(), 1);
        let id = empty[0].id;
        assert!(remove_room(&mut empty, id).is_err());
    }
}
