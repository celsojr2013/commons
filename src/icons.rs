use crate::*;
#[derive(Clone, Copy)]
pub enum Icon {
    Mic,
    Camera,
    Speaker,
    Settings,
    Call,
    Avatar,
    Theme,
    View,
    Server,
    Join,
    Edit,
    Lock,
    Fit,
    Plus,
    Minus,
    Up,
    Down,
    Close,
    Help,
}
pub fn paint(p: &Painter, icon: Icon, rect: Rect, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) / 20.0;
    let pt = |x: f32, y: f32| c + vec2(x * s, y * s);
    let stroke = Stroke::new(1.5 * s, color);
    let line = |a: (f32, f32), b: (f32, f32)| {
        p.line_segment([pt(a.0, a.1), pt(b.0, b.1)], stroke);
    };
    let box_ = |a: (f32, f32), b: (f32, f32), r: f32| {
        p.rect_stroke(
            Rect::from_min_max(pt(a.0, a.1), pt(b.0, b.1)),
            r * s,
            stroke,
            StrokeKind::Inside,
        );
    };
    match icon {
        Icon::Mic => {
            box_((-3.0, -8.0), (3.0, 3.0), 3.0);
            p.add(Shape::line(
                vec![
                    pt(-6.0, -1.0),
                    pt(-6.0, 3.0),
                    pt(-3.0, 6.0),
                    pt(3.0, 6.0),
                    pt(6.0, 3.0),
                    pt(6.0, -1.0),
                ],
                stroke,
            ));
            line((0.0, 6.0), (0.0, 9.0));
            line((-4.0, 9.0), (4.0, 9.0));
        }
        Icon::Camera => {
            box_((-8.0, -5.0), (3.0, 6.0), 2.0);
            p.add(Shape::closed_line(
                vec![pt(3.0, -2.0), pt(8.0, -5.0), pt(8.0, 6.0), pt(3.0, 3.0)],
                stroke,
            ));
        }
        Icon::Speaker => {
            p.add(Shape::closed_line(
                vec![
                    pt(-8.0, -3.0),
                    pt(-4.0, -3.0),
                    pt(1.0, -7.0),
                    pt(1.0, 7.0),
                    pt(-4.0, 3.0),
                    pt(-8.0, 3.0),
                ],
                stroke,
            ));
            p.add(Shape::line(
                vec![pt(5.0, -5.0), pt(8.0, -2.0), pt(8.0, 2.0), pt(5.0, 5.0)],
                stroke,
            ));
        }
        Icon::Settings => {
            for (x, y) in [(-5.0, -4.0), (0.0, 4.0), (5.0, -1.0)] {
                line((x, -8.0), (x, 8.0));
                p.circle_filled(pt(x, y), 2.0 * s, color);
            }
        }
        Icon::Call => {
            box_((-8.0, -7.0), (8.0, 5.0), 3.0);
            line((-4.0, 5.0), (-4.0, 9.0));
            line((-4.0, 9.0), (1.0, 5.0));
        }
        Icon::Avatar => {
            p.circle_stroke(pt(0.0, -4.0), 4.0 * s, stroke);
            p.add(Shape::line(
                vec![
                    pt(-8.0, 8.0),
                    pt(-6.0, 3.0),
                    pt(0.0, 1.0),
                    pt(6.0, 3.0),
                    pt(8.0, 8.0),
                ],
                stroke,
            ));
        }
        Icon::Theme => {
            p.circle_stroke(c, 7.0 * s, stroke);
            p.line_segment([pt(0.0, -7.0), pt(0.0, 7.0)], stroke);
            p.circle_filled(pt(-3.0, 0.0), 2.0 * s, color);
        }
        Icon::View => {
            box_((-8.0, -7.0), (8.0, 7.0), 2.0);
            line((-2.0, -7.0), (-2.0, 7.0));
            line((3.0, -2.0), (6.0, -2.0));
            line((3.0, 2.0), (6.0, 2.0));
        }
        Icon::Server => {
            box_((-8.0, -8.0), (8.0, -1.0), 2.0);
            box_((-8.0, 2.0), (8.0, 9.0), 2.0);
            p.circle_filled(pt(-4.0, -4.0), 1.0 * s, color);
            p.circle_filled(pt(-4.0, 5.0), 1.0 * s, color);
        }
        Icon::Join => {
            line((-8.0, 0.0), (4.0, 0.0));
            line((0.0, -4.0), (4.0, 0.0));
            line((0.0, 4.0), (4.0, 0.0));
            p.add(Shape::line(
                vec![pt(2.0, -8.0), pt(8.0, -8.0), pt(8.0, 8.0), pt(2.0, 8.0)],
                stroke,
            ));
        }
        Icon::Edit => {
            p.add(Shape::closed_line(
                vec![
                    pt(-7.0, 3.0),
                    pt(3.0, -7.0),
                    pt(7.0, -3.0),
                    pt(-3.0, 7.0),
                    pt(-8.0, 8.0),
                ],
                stroke,
            ));
            line((1.0, -5.0), (5.0, -1.0));
        }
        Icon::Lock => {
            box_((-7.0, -1.0), (7.0, 9.0), 2.0);
            box_((-4.0, -8.0), (4.0, 2.0), 4.0);
            p.circle_filled(pt(0.0, 4.0), 1.3 * s, color);
        }
        Icon::Fit => {
            for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                line((x * 8.0, y * 3.0), (x * 8.0, y * 8.0));
                line((x * 8.0, y * 8.0), (x * 3.0, y * 8.0));
            }
        }
        Icon::Plus => {
            line((-7.0, 0.0), (7.0, 0.0));
            line((0.0, -7.0), (0.0, 7.0));
        }
        Icon::Minus => line((-7.0, 0.0), (7.0, 0.0)),
        Icon::Up => {
            line((-5.0, 2.0), (0.0, -3.0));
            line((0.0, -3.0), (5.0, 2.0));
        }
        Icon::Down => {
            line((-5.0, -2.0), (0.0, 3.0));
            line((0.0, 3.0), (5.0, -2.0));
        }
        Icon::Close => {
            line((-5.0, -5.0), (5.0, 5.0));
            line((-5.0, 5.0), (5.0, -5.0));
        }
        Icon::Help => {
            p.circle_stroke(c, 8.0 * s, stroke);
            p.text(
                c,
                Align2::CENTER_CENTER,
                "?",
                FontId::proportional(14.0 * s),
                color,
            );
        }
    }
}
pub fn button(ui: &mut Ui, icon: Icon, label: &str, active: bool, enabled: bool) -> Response {
    let p = theme::palette(ui.ctx());
    let response = ui.add_enabled(
        enabled,
        Button::new(RichText::new(format!("     {label}")).size(12.0))
            .fill(if active { p.soft } else { p.raised })
            .min_size(vec2(0.0, 34.0))
            .corner_radius(7),
    );
    let color = if enabled {
        if active {
            p.accent
        } else {
            p.text
        }
    } else {
        p.muted.gamma_multiply(0.65)
    };
    paint(
        ui.painter(),
        icon,
        Rect::from_center_size(
            response.rect.left_center() + vec2(15.0, 0.0),
            Vec2::splat(17.0),
        ),
        color,
    );
    response
}
pub fn small(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    let response = ui.add(Button::new("").min_size(Vec2::splat(27.0)).corner_radius(6));
    paint(
        ui.painter(),
        icon,
        Rect::from_center_size(response.rect.center(), Vec2::splat(15.0)),
        theme::palette(ui.ctx()).text,
    );
    response.on_hover_text(tooltip)
}
