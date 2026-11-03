use crate::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Mode {
    #[default]
    Light,
    Dark,
    System,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Scheme {
    #[default]
    Lavender,
    Ocean,
    Forest,
    Rose,
    Amber,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Appearance {
    pub mode: Mode,
    pub scheme: Scheme,
}
#[derive(Clone, Copy)]
pub struct Palette {
    pub panel: Color32,
    pub background: Color32,
    pub sidebar: Color32,
    pub raised: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub soft: Color32,
    pub primary_text: Color32,
}
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgb(
        (a.r() as f32 * (1.0 - t) + b.r() as f32 * t) as u8,
        (a.g() as f32 * (1.0 - t) + b.g() as f32 * t) as u8,
        (a.b() as f32 * (1.0 - t) + b.b() as f32 * t) as u8,
    )
}
pub fn palette(ctx: &Context) -> Palette {
    let visuals = ctx.style().visuals.clone();
    colors(visuals.dark_mode, visuals.selection.stroke.color)
}
fn colors(dark: bool, accent: Color32) -> Palette {
    if dark {
        let panel = Color32::from_rgb(31, 34, 48);
        Palette {
            panel,
            background: Color32::from_rgb(20, 22, 32),
            sidebar: Color32::from_rgb(26, 28, 40),
            raised: Color32::from_rgb(40, 44, 60),
            border: Color32::from_rgb(57, 62, 82),
            text: Color32::from_rgb(237, 239, 249),
            muted: Color32::from_rgb(167, 173, 194),
            accent,
            soft: mix(panel, accent, 0.17),
            primary_text: Color32::from_rgb(20, 22, 32),
        }
    } else {
        let panel = Color32::WHITE;
        Palette {
            panel,
            background: Color32::from_rgb(245, 246, 250),
            sidebar: Color32::from_rgb(250, 250, 253),
            raised: Color32::from_rgb(245, 245, 250),
            border: Color32::from_rgb(227, 230, 238),
            text: Color32::from_rgb(36, 38, 49),
            muted: Color32::from_rgb(119, 125, 146),
            accent,
            soft: mix(panel, accent, 0.13),
            primary_text: Color32::WHITE,
        }
    }
}
fn accent(s: Scheme, dark: bool) -> Color32 {
    let c = match (s, dark) {
        (Scheme::Lavender, false) => (116, 96, 226),
        (Scheme::Lavender, true) => (179, 162, 255),
        (Scheme::Ocean, false) => (35, 117, 161),
        (Scheme::Ocean, true) => (112, 199, 239),
        (Scheme::Forest, false) => (38, 122, 87),
        (Scheme::Forest, true) => (112, 211, 160),
        (Scheme::Rose, false) => (180, 67, 112),
        (Scheme::Rose, true) => (242, 153, 193),
        (Scheme::Amber, false) => (153, 94, 26),
        (Scheme::Amber, true) => (239, 195, 113),
    };
    Color32::from_rgb(c.0, c.1, c.2)
}
pub fn apply(ctx: &Context, appearance: Appearance) {
    let dark = match appearance.mode {
        Mode::Dark => true,
        Mode::Light => false,
        Mode::System => ctx.system_theme() == Some(egui::Theme::Dark),
    };
    let key = (appearance, dark);
    let id = Id::new("commons appearance");
    let expected_accent = accent(appearance.scheme, dark);
    if ctx.data(|d| d.get_temp::<(Appearance, bool)>(id)) == Some(key)
        && ctx.style().visuals.selection.stroke.color == expected_accent
        && ctx.style().spacing.button_padding == vec2(14.0, 10.0)
        && ctx.style().visuals.dark_mode == dark
    {
        return;
    }
    ctx.set_theme(if dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    });
    let p = colors(dark, expected_accent);
    let mut style = (*ctx.style()).clone();
    style.visuals = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    style.visuals.override_text_color = Some(p.text);
    style.visuals.panel_fill = p.panel;
    style.visuals.window_fill = p.panel;
    style.visuals.extreme_bg_color = p.background;
    style.visuals.faint_bg_color = p.raised;
    style.visuals.window_stroke = Stroke::new(1.0, p.border);
    style.visuals.hyperlink_color = p.accent;
    style.visuals.selection.bg_fill = p.soft;
    style.visuals.selection.stroke = Stroke::new(1.0, p.accent);
    for widget in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.noninteractive,
    ] {
        widget.bg_fill = p.raised;
        widget.weak_bg_fill = p.raised;
        widget.bg_stroke = Stroke::new(1.0, p.border);
        widget.fg_stroke = Stroke::new(1.0, p.text);
    }
    style.visuals.widgets.hovered.bg_fill = p.soft;
    style.visuals.widgets.hovered.weak_bg_fill = p.soft;
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, p.text);
    style.visuals.widgets.active.bg_fill = p.soft;
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, p.text);
    style.spacing.item_spacing = vec2(10.0, 12.0);
    style.spacing.scroll = egui::style::ScrollStyle::solid();
    style.spacing.button_padding = vec2(14.0, 10.0);
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(14.0));
    style
        .text_styles
        .insert(TextStyle::Button, FontId::proportional(14.0));
    ctx.set_style(style);
    ctx.data_mut(|d| d.insert_temp(id, key));
}
impl Commons {
    pub fn appearance_dialog(&mut self, ctx: &Context) {
        let mut open = true;
        Window::new("Appearance").open(&mut open).collapsible(false).resizable(false).default_width(440.0).anchor(Align2::CENTER_CENTER,Vec2::ZERO).show(ctx,|ui|{ui.label(RichText::new("Make the workspace comfortable for you.").color(palette(ctx).muted));let before=self.settings.appearance;ui.horizontal(|ui|{let r=ui.selectable_value(&mut self.settings.appearance.mode,Mode::Light,"Light");self.target("theme_light",r.rect);let r=ui.selectable_value(&mut self.settings.appearance.mode,Mode::Dark,"Dark");self.target("theme_dark",r.rect);let r=ui.selectable_value(&mut self.settings.appearance.mode,Mode::System,"System");self.target("theme_system",r.rect);});ui.add_space(10.0);ui.label("Color scheme");for(s,name)in[(Scheme::Lavender,"Lavender"),(Scheme::Ocean,"Ocean"),(Scheme::Forest,"Forest"),(Scheme::Rose,"Rose"),(Scheme::Amber,"Amber")]{ui.horizontal(|ui|{let(r,response)=ui.allocate_exact_size(vec2(24.0,24.0),Sense::click());ui.painter().circle_filled(r.center(),9.0,accent(s,ui.visuals().dark_mode));if response.clicked(){self.settings.appearance.scheme=s;}let r=ui.selectable_value(&mut self.settings.appearance.scheme,s,name);if s==Scheme::Ocean{self.target("theme_ocean",r.rect);}});}if before!=self.settings.appearance{apply(ctx,self.settings.appearance);self.save_settings();}ui.add_space(8.0);ui.label(RichText::new("Your preference is saved on this computer. Tile and character artwork keeps its original colors.").size(11.0).color(palette(ctx).muted));let r=ui.button("Done");self.target("theme_done",r.rect);if r.clicked(){self.dialog=None;}});
        if !open {
            self.dialog = None;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn appearance_recovers_when_the_web_runner_replaces_the_initial_style() {
        let ctx = Context::default();
        let appearance = Appearance {
            mode: Mode::Dark,
            scheme: Scheme::Ocean,
        };
        apply(&ctx, appearance);
        ctx.set_style(Style::default());
        apply(&ctx, appearance);
        assert!(ctx.style().visuals.dark_mode);
        assert_eq!(palette(&ctx).accent, accent(Scheme::Ocean, true));
        assert_eq!(ctx.style().spacing.button_padding, vec2(14.0, 10.0));
    }
    #[test]
    fn every_palette_has_distinct_surface_and_readable_text() {
        let ctx = Context::default();
        for mode in [Mode::Light, Mode::Dark] {
            for scheme in [
                Scheme::Lavender,
                Scheme::Ocean,
                Scheme::Forest,
                Scheme::Rose,
                Scheme::Amber,
            ] {
                apply(&ctx, Appearance { mode, scheme });
                let p = palette(&ctx);
                assert_ne!(p.panel, p.text);
                assert_ne!(p.accent, p.panel);
                assert_eq!(ctx.style().visuals.dark_mode, mode == Mode::Dark);
                let serialized = serde_json::to_string(&Appearance { mode, scheme }).unwrap();
                assert_eq!(
                    serde_json::from_str::<Appearance>(&serialized).unwrap(),
                    Appearance { mode, scheme }
                );
            }
        }
    }
}
