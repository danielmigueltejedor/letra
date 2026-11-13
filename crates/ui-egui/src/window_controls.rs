//! Nodalia's window controls on Linux and the BSDs, where the window has no OS decorations
//! ([`WordApp::ndl_window_controls`]): the title bar is the window's own. Its free stretch drags
//! the window and a double-click there maximizes it (`chrome::title_bar`); round 28 pt buttons
//! minimize, maximize or restore, and close, and thin invisible zones along the window's edges
//! resize it. macOS and Windows keep their native controls, and File › Options › General ›
//! Window controls brings the desktop's own back on Linux.

use egui::{CursorIcon, Id, LayerId, Order, PointerButton, Pos2, Rect, ResizeDirection, Sense, Stroke, Ui, ViewportCommand, pos2, vec2};

use crate::WordApp;
use crate::theme::{self, Tokens};

/// Resize zone thickness along an edge, and the side of a corner zone.
const EDGE: f32 = 8.0;
const CORNER: f32 = 12.0;
/// A button's side, the gap on each side of it, and the gap between the last one and the window's
/// edge (NDL: 28 pt circles, 4 pt apart, 8 pt from the edge).
const SIZE: f32 = 28.0;
const GAP: f32 = 2.0;
const END: f32 = 8.0;

/// Which window controls Letra draws on Linux and the BSDs (File › Options › General).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindowControls {
    /// Nodalia's round buttons in Letra's own title bar.
    #[default]
    Nodalia,
    /// The desktop's title bar and buttons.
    System,
}

impl WindowControls {
    pub const ALL: [WindowControls; 2] = [WindowControls::Nodalia, WindowControls::System];

    pub fn code(self) -> &'static str {
        match self {
            WindowControls::Nodalia => "nodalia",
            WindowControls::System => "system",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.code().eq_ignore_ascii_case(code.trim()))
    }

    /// The English label (translated with `tl!` where shown).
    pub fn label(self) -> &'static str {
        match self {
            WindowControls::Nodalia => "Nodalia",
            WindowControls::System => "Use the desktop's",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Caption {
    Minimize,
    Maximize,
    Close,
}

const CAPTIONS: [Caption; 3] = [Caption::Minimize, Caption::Maximize, Caption::Close];

/// The width the buttons take at the title bar's right end: zero without Nodalia's controls.
pub fn reserved(app: &WordApp) -> f32 {
    if app.ndl_window_controls { CAPTIONS.len() as f32 * (SIZE + 2.0 * GAP) + END - GAP } else { 0.0 }
}

/// Caption button ids (stable, so tests and agents can find them).
pub fn caption_id(c: Caption) -> Id {
    Id::unique(("window-caption", c as u8))
}

fn maximized(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.viewport().maximized.unwrap_or(false))
}

/// Maximize the window, or restore it when it already is.
pub fn toggle_maximize(ctx: &egui::Context) {
    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized(ctx)));
}

/// The buttons at the right end of `bar` (the title bar's rect), full bar height. Close asks the
/// host to close the window, which asks about unsaved work first. The buttons drag the window too,
/// as GTK's and KDE's do.
pub fn caption_buttons(app: &mut WordApp, ui: &mut Ui, bar: Rect) {
    if !app.ndl_window_controls {
        return;
    }
    let t = Tokens::get(ui.ctx());
    let max = maximized(ui.ctx());
    let right = bar.right().min(ui.ctx().content_rect().right());
    let mut x = right - reserved(app);
    let mut clicked = None;
    for (i, c) in CAPTIONS.into_iter().enumerate() {
        let after = if i + 1 == CAPTIONS.len() { END } else { GAP };
        let r = Rect::from_min_max(pos2(x, bar.top()), pos2(x + GAP + SIZE + after, bar.bottom()));
        let centre = pos2(x + GAP + SIZE / 2.0, bar.center().y);
        x = r.right();
        let resp = ui.interact(r, caption_id(c), Sense::click_and_drag());
        if resp.drag_started_by(PointerButton::Primary) {
            start_os_move(ui.ctx());
        }
        let pressed = resp.is_pointer_button_down_on() && resp.hovered();
        paint(ui, &t, c, max, centre, (resp.hovered(), pressed), 1.0);
        let label = match c {
            Caption::Minimize => tl!("Minimize"),
            Caption::Maximize if max => tl!("Restore"),
            Caption::Maximize => tl!("Maximize"),
            Caption::Close => tl!("Close"),
        };
        // The glyphs say what the buttons do: no tooltip repeats it.
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
        if resp.clicked() {
            clicked = Some(c);
        }
    }
    match clicked {
        Some(Caption::Minimize) => ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true)),
        Some(Caption::Maximize) => toggle_maximize(ui.ctx()),
        Some(Caption::Close) => ui.ctx().send_viewport_cmd(ViewportCommand::Close),
        None => {}
    }
}

/// A Nodalia window button centred at `centre`: a raised disc at rest, then the tint of its kind
/// under the pointer (the accent for minimize, amber for maximize, coral for close), and NDL's
/// window glyph drawn on its 24 pt grid with 2 pt strokes, all scaled by `k` (1 for the window's
/// own 28 pt buttons, smaller for a dialog's close button).
pub(crate) fn paint(ui: &Ui, t: &Tokens, c: Caption, maximized: bool, centre: Pos2, (hovered, pressed): (bool, bool), k: f32) {
    let p = theme::palette(if t.dark { nodalia_theme::Mode::Dark } else { nodalia_theme::Mode::Light });
    let ndl = theme::ndl;
    let ppp = ui.ctx().pixels_per_point();
    let centre = pos2((centre.x * ppp).round() / ppp, (centre.y * ppp).round() / ppp);
    let (mut fill, ink) = match (c, hovered || pressed) {
        (Caption::Close, true) => (t.close_hover, t.on_close),
        (Caption::Maximize, true) => (ndl(p.status_warning_bg), ndl(p.status_warning_fg)),
        (Caption::Minimize, true) => (ndl(p.accent_subtle), ndl(p.accent_text)),
        _ => (ndl(p.control_bg), ndl(p.text_secondary)),
    };
    if pressed {
        fill = fill.gamma_multiply(0.82).to_opaque();
    }
    let painter = ui.painter();
    painter.circle_filled(centre, 14.0 * k, fill);
    let at = |x: f32, y: f32| centre + vec2(x - 12.0, y - 12.0) * k;
    let s = Stroke::new(2.0 * k, ink);
    let round = |q: Pos2| painter.circle_filled(q, k, ink);
    let path = |points: &[Pos2]| {
        if let (Some(&a), Some(&b)) = (points.first(), points.last()) {
            round(a);
            round(b);
        }
        for w in points.windows(2) {
            if let [a, b] = w {
                round(*a);
                painter.line_segment([*a, *b], s);
            }
        }
    };
    match c {
        Caption::Minimize => path(&[at(8.0, 12.0), at(16.0, 12.0)]),
        // Two corners on the diagonal: open outwards to maximize, turned to the centre to restore.
        Caption::Maximize if maximized => {
            path(&[at(14.0, 7.0), at(14.0, 10.0), at(17.0, 10.0)]);
            path(&[at(10.0, 17.0), at(10.0, 14.0), at(7.0, 14.0)]);
        }
        Caption::Maximize => {
            path(&[at(12.0, 7.0), at(17.0, 7.0), at(17.0, 12.0)]);
            path(&[at(12.0, 17.0), at(7.0, 17.0), at(7.0, 12.0)]);
        }
        Caption::Close => {
            path(&[at(8.0, 8.0), at(16.0, 16.0)]);
            path(&[at(16.0, 8.0), at(8.0, 16.0)]);
        }
    }
}

/// A dialog's close button in the Nodalia look: the window buttons' disc and ×, filling `rect`.
/// Says whether it was clicked.
pub fn close_button(ui: &mut Ui, rect: Rect, id: Id) -> bool {
    let t = Tokens::get(ui.ctx());
    let resp = ui.interact(rect, id, Sense::click());
    let pressed = resp.is_pointer_button_down_on() && resp.hovered();
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    child.set_clip_rect(ui.clip_rect().union(rect.expand(2.0)));
    paint(&child, &t, Caption::Close, false, rect.center(), (resp.hovered(), pressed), rect.height() / 28.0);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tl!("Close")));
    resp.clicked()
}

/// The edge and corner zones of a window with content rect `w`: (zone, direction, cursor).
/// Corners come last so they win where they overlap the edges.
fn zones(w: Rect) -> [(Rect, ResizeDirection, CursorIcon); 8] {
    use ResizeDirection::*;
    let (e, c) = (EDGE, CORNER);
    [
        (Rect::from_min_max(w.left_top(), pos2(w.right(), w.top() + e)), North, CursorIcon::ResizeNorth),
        (Rect::from_min_max(pos2(w.left(), w.bottom() - e), w.right_bottom()), South, CursorIcon::ResizeSouth),
        (Rect::from_min_max(w.left_top(), pos2(w.left() + e, w.bottom())), West, CursorIcon::ResizeWest),
        (Rect::from_min_max(pos2(w.right() - e, w.top()), w.right_bottom()), East, CursorIcon::ResizeEast),
        (Rect::from_min_size(w.left_top(), vec2(c, c)), NorthWest, CursorIcon::ResizeNwSe),
        (Rect::from_min_size(w.right_top() - vec2(c, 0.0), vec2(c, c)), NorthEast, CursorIcon::ResizeNeSw),
        (Rect::from_min_size(w.left_bottom() - vec2(0.0, c), vec2(c, c)), SouthWest, CursorIcon::ResizeNeSw),
        (Rect::from_min_size(w.right_bottom() - vec2(c, c), vec2(c, c)), SouthEast, CursorIcon::ResizeNwSe),
    ]
}

fn handed_off_id() -> Id {
    Id::unique("window-controls-handed-off")
}

/// An OS resize or move started at `pass`; `released` is the pass whose input ended the press.
#[derive(Clone, Copy, Debug)]
struct HandOff {
    pass: u64,
    released: Option<u64>,
}

fn hand_off(ctx: &egui::Context) {
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(handed_off_id(), HandOff { pass, released: None }));
}

/// Move the window from this press. The window manager keeps the release of an OS move, so the
/// press is ended by [`release_after_os_move`].
pub fn start_os_move(ctx: &egui::Context) {
    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
    hand_off(ctx);
}

/// The compositor takes the button release of an OS resize or move (KWin, Mutter), so egui would
/// keep the press: every other move or resize failed. End it on the input after the hand-off, only
/// while egui still has it down and the input brings no release of its own. Call from
/// `raw_input_hook`.
pub fn release_after_os_move(ctx: &egui::Context, raw: &mut egui::RawInput) {
    let Some(h) = ctx.data(|d| d.get_temp::<HandOff>(handed_off_id())) else { return };
    if h.released.is_some() || h.pass >= ctx.cumulative_pass_nr() {
        return;
    }
    let os_released = raw.events.iter().any(|e| matches!(e, egui::Event::PointerButton { button: PointerButton::Primary, pressed: false, .. }));
    if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()).filter(|_| !os_released && ctx.input(|i| i.pointer.primary_down())) {
        raw.events.push(egui::Event::PointerButton { pos, button: PointerButton::Primary, pressed: false, modifiers: ctx.input(|i| i.modifiers) });
    }
    let pass = ctx.cumulative_pass_nr();
    ctx.data_mut(|d| d.insert_temp(handed_off_id(), HandOff { released: Some(pass), ..h }));
}

/// The compositor sets its own cursor during an OS resize, and egui-winit only re-sends a cursor
/// that changed: on the first motion after the hand-off, show the default one for a frame.
fn reshow_cursor(ctx: &egui::Context) {
    let Some(HandOff { released: Some(pass), .. }) = ctx.data(|d| d.get_temp::<HandOff>(handed_off_id())) else { return };
    if pass < ctx.cumulative_pass_nr() && ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::PointerMoved(_)))) {
        ctx.data_mut(|d| d.remove::<HandOff>(handed_off_id()));
        ctx.set_cursor_icon(CursorIcon::Default);
        ctx.request_repaint();
    }
}

/// Invisible resize zones along the window's edges, above everything else; a press in one starts
/// an OS resize. None while the window is maximized or full screen, or has OS decorations.
pub fn resize_zones(app: &WordApp, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    if !app.ndl_window_controls || maximized(&ctx) || ctx.input(|i| i.viewport().fullscreen.unwrap_or(false)) {
        reshow_cursor(&ctx);
        return;
    }
    let w = ctx.content_rect();
    let layer = LayerId::new(Order::Foreground, Id::unique("window-resize"));
    let zui = ui.new_child(egui::UiBuilder::new().layer_id(layer).max_rect(w));
    for (i, (r, dir, cursor)) in zones(w).into_iter().enumerate() {
        let resp = zui.interact(r, Id::unique(("window-resize", i)), Sense::drag()).on_hover_cursor(cursor);
        if resp.drag_started_by(PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
            hand_off(&ctx);
        }
    }
    reshow_cursor(&ctx);
}
