//! Add-ons page, the "+" at the end of the tab row: features only the people
//! who switch them on get (`poni_core::addons`). Most download then; Script
//! reader, System, Health and Stocks are already in the app and only show
//! their pages (System's readings, Health's steps and Stocks' prices also run
//! only while they are on).
//!
//! One row per add-on: its icon, its name, what it does (or where its
//! download stands, with a bar while it comes in), its size while it is off,
//! ("Built in" for one with nothing to download), and a switch. The whole
//! row toggles, as in Settings: on downloads the add-on and starts it, off
//! stops it and deletes its files
//! (`Command::SaveSettings`; the app passes the change to `services::addons`).
//! Both apps list every add-on ([`OFFERED`]).

use crate::commands::Command;
use crate::gfx::TextStyle;
use crate::icons::Icon;
use crate::state::AppState;
use crate::ui::{wid_n, Ui};
use poni_core::addons::{self, Addon, Status};
use poni_core::geom::{palette, Color, Point, Rect};
use poni_core::phonedrop::fmt_bytes;

const INTRO_H: f32 = 16.0;
const ROWS_Y: f32 = 24.0;
/// Rows shrink to fit [`FIT_H`], but never below `ROW_MIN` (text stays legible)
/// nor above `ROW_MAX`.
const ROW_MAX: f32 = 45.0;
const ROW_MIN: f32 = 38.0;
const FIT_H: f32 = 256.0;
const BADGE_MAX: f32 = 36.0;
const TEXT_X: f32 = 48.0;
const TITLE_H: f32 = 19.0;
const LINE_H: f32 = 15.0;
/// A row's hover wash reaches this far past its contents (as in Settings).
const ROW_BLEED: f32 = 8.0;
const SWITCH_W: f32 = 30.0;
const SWITCH_H: f32 = 18.0;
const BAR_H: f32 = 3.0;

/// The add-ons this app has, in the page's order: all of them, on Windows
/// and on the Mac (whose Prayer times use macOS's location and sounds).
pub const OFFERED: &[&Addon] = &addons::ALL;
/// The lines name the Mac ("on this Mac") and leave out Windows' reasons.
const MAC: bool = cfg!(target_os = "macos");

pub fn height(_st: &AppState) -> f32 {
    let n = OFFERED.len();
    ROWS_Y + n as f32 * row_h(n)
}

fn row_h(n: usize) -> f32 {
    ((FIT_H - ROWS_Y) / n.max(1) as f32).floor().clamp(ROW_MIN, ROW_MAX)
}

/// Each add-on's icon on its badge.
fn icon(a: &Addon) -> Icon {
    match a.id {
        "sholat" => Icon::Moon,
        "ja" => Icon::Lyrics,
        "script" => Icon::Script,
        "system" => Icon::Gauge,
        "health" => Icon::Steps,
        "stocks" => Icon::Stocks,
        "reminders" => Icon::Clock,
        _ => Icon::Addons,
    }
}

pub fn draw(ui: &mut Ui, st: &mut AppState, area: Rect) {
    let n = OFFERED.len();
    let on_count = OFFERED.iter().filter(|a| st.settings.addon_on(a.id)).count();

    // Header: a short intro on the left, "3 of 7 on" on the right.
    let count = format!("{on_count} of {n} on");
    let right = TextStyle::footnote().color(palette::TEXT_DIM).right();
    let cw = ui.c.measure(&count, &right, 200.0).0.ceil();
    let left = TextStyle::footnote().color(palette::TEXT_DIM);
    let intro = "Off until you switch it on. Switching off removes its download.";
    ui.c.text(intro, Rect::new(area.x, area.y, (area.w - cw - 8.0).max(0.0), INTRO_H), &left);
    ui.c.text(&count, Rect::new(area.right() - cw, area.y, cw, INTRO_H), &right);

    // The last row's hover wash meets the Pill card's rounded corners.
    let card = super::card::pill_card(st, area);
    let h = row_h(n);
    for (i, a) in OFFERED.iter().enumerate() {
        let row = Rect::new(area.x, area.y + ROWS_Y + i as f32 * h, area.w, h);
        if addon_row(ui, st, row, i, a, card) {
            let on = st.settings.addon_on(a.id);
            st.settings.set_addon(a.id, !on);
            st.cmd(Command::SaveSettings);
        }
    }
}

/// Returns true when the row was clicked. `card`: the Pill card (`card::pill_card`).
fn addon_row(ui: &mut Ui, st: &AppState, r: Rect, i: usize, a: &Addon, card: Option<(Rect, f32)>) -> bool {
    let status = st.addon(a.id);
    let on = status.is_on();
    let accent = st.accent;
    let hit = Rect::new(r.x - ROW_BLEED, r.y + 2.0, r.w + 2.0 * ROW_BLEED, r.h - 4.0);
    let resp = ui.button(wid_n("addons.row", i as u64), hit);
    if resp.hovered || resp.pressed {
        super::card::fill_box(ui, card, hit, 10.0, Color::white(if resp.pressed { 0.10 } else { 0.06 }));
    }
    let cy = r.center().y;

    // The badge scales with the row, centred in a fixed column so the text
    // never shifts. Only an add-on that is on wears the accent.
    let badge = (r.h * 0.78).round().min(BADGE_MAX);
    let bx = r.x + BADGE_MAX / 2.0;
    let (fill, ink) = if on {
        (accent.with_alpha(0.22), accent)
    } else {
        (Color::white(0.07), palette::TEXT_DIM)
    };
    ui.c.fill_circle(Point::new(bx, cy), badge / 2.0, fill);
    let glyph = (badge * 0.5).round();
    ui.c.icon(icon(a), Rect::from_center(Point::new(bx, cy), glyph, glyph), ink);

    let sw = Rect::new(r.right() - SWITCH_W, cy - SWITCH_H / 2.0, SWITCH_W, SWITCH_H);
    crate::pages::settings::draw_switch(ui.c, sw, on, accent);
    // The size, while there is a choice to make.
    let mut text_right = sw.x - 16.0;
    if status == Status::Off {
        let size = if a.downloads() { fmt_bytes(a.size()) } else { "Built in".to_string() };
        let style = TextStyle::footnote().color(palette::TEXT_DIM).right();
        let w = ui.c.measure(&size, &style, 200.0).0.ceil();
        ui.c.text(&size, Rect::new(sw.x - 10.0 - w, r.y, w, r.h), &style);
        text_right = sw.x - 10.0 - w - 16.0;
    }

    // Title + line, vertically centred; lifted a little while a bar is shown.
    let x = r.x + TEXT_X;
    let w = (text_right - x).max(0.0);
    let progress = status.progress();
    let lift = if progress.is_some() { 2.0 } else { 0.0 };
    let top = r.y + ((r.h - (TITLE_H + LINE_H)) / 2.0).floor() - lift;
    ui.c.text(a.title, Rect::new(x, top, w, TITLE_H), &TextStyle::headline());
    let line_color = if status == Status::Failed { palette::ORANGE } else { palette::TEXT_DIM };
    ui.c.text(
        &status.line(a, MAC),
        Rect::new(x, top + TITLE_H, w, LINE_H),
        &TextStyle::footnote().color(line_color),
    );
    if let Some(p) = progress {
        let bar = Rect::new(x, r.bottom() - BAR_H - 1.0, w, BAR_H);
        ui.c.fill_rrect(bar, BAR_H / 2.0, palette::TRACK);
        if p > 0.0 {
            ui.c.fill_rrect(Rect::new(bar.x, bar.y, (bar.w * p).max(BAR_H), BAR_H), BAR_H / 2.0, accent);
        }
    }
    resp.clicked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_stay_readable_and_up_to_six_fit() {
        for n in 1..=6 {
            assert!(ROWS_Y + n as f32 * row_h(n) <= FIT_H, "n = {n}");
        }
        for n in 1..=12 {
            let h = row_h(n);
            assert!((ROW_MIN..=ROW_MAX).contains(&h), "n = {n}, h = {h}");
        }
    }
}