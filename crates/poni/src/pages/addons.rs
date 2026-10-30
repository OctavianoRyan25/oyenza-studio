//! Add-ons page, the "+" at the end of the tab row: features only the people
//! who switch them on get (`poni_core::addons`). Most download then; Script
//! reader, System, Health, Stocks, Notifications and IELTS are already in
//! the app and only show their pages (System's readings, Health's steps,
//! Stocks' prices and Notifications' listening also run only while they are
//! on; IELTS only while a session runs).
//!
//! One row per add-on: a badge (accent while it is on, neutral while off), its
//! name, what it does (or where its download stands, with a thin bar along the
//! row's bottom edge while it comes in), its size while it is off ("Built in"
//! for one with nothing to download), and a switch. The whole row toggles, as
//! in Settings: on downloads the add-on and starts it, off stops it and
//! deletes its files (`Command::SaveSettings`; the app passes the change to
//! `services::addons`). One whose files this build can't get yet stays off and
//! says so (`Status::Unpinned`).
//!
//! Both apps list every add-on ([`OFFERED`]) but Screen recorder, which shows
//! once Oyenza Studio Recorder is installed, and Notifications, which only
//! Windows has ([`rows`]). Rows scale between `ROW_MIN` and `ROW_MAX` so the
//! page keeps the island's height; seven or more are one line each: the name,
//! then the line after it.
//!
//! A Studio Pro add-on's row says Pro where its size was. IELTS and the
//! Downloader are listed only for an account with Pro ([`rows`]); the
//! Downloader's tools come only with it ([`needs_pro`]).

use crate::commands::Command;
use crate::gfx::{TextStyle, Weight};
use crate::icons::Icon;
use crate::state::AppState;
use crate::ui::{wid_n, Ui};
use poni_core::addons::{self, Addon, Status};
use poni_core::geom::{palette, Color, Point, Rect};
use poni_core::phonedrop::fmt_bytes;

const INTRO_H: f32 = 16.0;
const ROWS_Y: f32 = 24.0;
/// Rows shrink to fit [`FIT_H`], but never below `ROW_MIN` (a line each, still
/// legible) nor above `ROW_MAX`.
const ROW_MAX: f32 = 45.0;
const ROW_MIN: f32 = 20.0;
const FIT_H: f32 = 256.0;
const MAX_H: f32 = FIT_H + INTRO_H + ROWS_Y;
const BADGE_MAX: f32 = 36.0;
const TEXT_X: f32 = 48.0;
const TITLE_H: f32 = 19.0;
const LINE_H: f32 = 15.0;
/// A row's hover wash reaches this far past its contents (as in Settings).
const ROW_BLEED: f32 = 8.0;
const SWITCH_W: f32 = 30.0;
const SWITCH_H: f32 = 18.0;
const BAR_H: f32 = 3.0;
/// Rows shorter than this (seven or more of them: 33 DIPs and less, where the
/// name and the line would overlap) put the line after the name.
const TWO_LINES: f32 = TITLE_H + LINE_H;
const BADGE_ONE_LINE: f32 = 24.0;
/// Between the name and the line after it, on a one-line row.
const LINE_GAP: f32 = 10.0;

/// The add-ons this app has, in the page's order: all of them, on Windows
/// and on the Mac (whose Prayer times use macOS's location and sounds).
pub const OFFERED: &[&Addon] = &addons::ALL;
/// The lines name the Mac ("on this Mac") and leave out Windows' reasons.
const MAC: bool = cfg!(target_os = "macos");

pub fn height(st: &AppState) -> f32 {
    let n = rows(st).len();
    ROWS_Y + n as f32 * row_h(n)
}

/// The page's rows: every add-on offered, Screen recorder only once Oyenza
/// Studio Recorder is installed and has spoken to the island
/// (`AppState::recorder_ready`), Notifications only on Windows (a Mac has
/// no way to read other apps' notifications), IELTS only for an account the
/// server says has Studio Pro, Downloader only where it's offered
/// (`AppState::downloader_offered`).
fn rows(st: &AppState) -> Vec<&'static Addon> {
    OFFERED
        .iter()
        .copied()
        .filter(|a| a.id != addons::SCREEN_RECORDER.id || st.recorder_ready())
        .filter(|a| a.id != addons::NOTIFICATIONS.id || cfg!(windows))
        .filter(|a| a.id != addons::DOWNLOADER.id || (st.downloader_offered && st.studio_pro() == Some(true)))
        .filter(|a| a.id != addons::IELTS.id || st.studio_pro() == Some(true))
        .collect()
}

/// The Pro add-on whose switch works only with Studio Pro, since its tab
/// needs it (`AppState::downloader_on`): the Downloader, whose tools download
/// only for Pro. Meeting notes switches on for anyone, and its page says what
/// Pro is.
fn needs_pro(a: &Addon) -> bool {
    a.id == addons::DOWNLOADER.id
}

/// It is switched on where it's offered: the island asks the server about
/// Pro as soon as it's signed in (at start, or just connected), so its tab
/// doesn't wait for this page to open.
pub fn pro_wanted(st: &AppState) -> bool {
    st.downloader_offered && st.settings.addon_on(addons::DOWNLOADER.id)
}

fn row_h(n: usize) -> f32 {
    ((FIT_H - ROWS_Y) / n.max(1) as f32).floor().clamp(ROW_MIN, ROW_MAX)
}

/// Where `a` stands: Screen recorder follows the recorder's install
/// (`Settings::recorder_on`); the Downloader is off without Pro, even
/// switched on before; the others, their own switch and download.
fn status(st: &AppState, a: &Addon) -> Status {
    if a.id == addons::SCREEN_RECORDER.id {
        if st.record_on() {
            Status::Ready
        } else {
            Status::Off
        }
    } else if needs_pro(a) && st.studio_pro() != Some(true) {
        Status::Off
    } else {
        st.addon(a.id)
    }
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
        "notifications" => Icon::Bell,
        "recorder" => Icon::Recorder,
        "downloader" => Icon::Download,
        "ielts" => Icon::Mic,
        "notes" => Icon::Notes,
        _ => Icon::Addons,
    }
}

pub fn draw(ui: &mut Ui, st: &mut AppState, area: Rect) {
    let list = rows(st);
    let n = list.len();
    let on_count = list.iter().filter(|a| status(st, a).is_on()).count();

    // Header: a short intro on the left, "3 of 9 on" on the right.
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
    for (i, a) in list.iter().enumerate() {
        let row = Rect::new(area.x, area.y + ROWS_Y + i as f32 * h, area.w, h);
        if addon_row(ui, st, row, i, a, card) {
            let on = status(st, a).is_on();
            if !on && !a.pinned() {
                // This build can't get its files: it stays off, and its line says why.
                st.addons.insert(a.id, Status::Unpinned);
            } else {
                st.settings.set_addon(a.id, !on);
                st.cmd(Command::SaveSettings);
            }
        }
    }
}

/// Returns true when the row was clicked. `card`: the Pill card (`card::pill_card`).
fn addon_row(ui: &mut Ui, st: &AppState, r: Rect, i: usize, a: &Addon, card: Option<(Rect, f32)>) -> bool {
    let status = status(st, a);
    let on = status.is_on();
    let accent = st.accent;
    let one_line = r.h < TWO_LINES;
    let hit = Rect::new(r.x - ROW_BLEED, r.y + 2.0, r.w + 2.0 * ROW_BLEED, r.h - 4.0);
    let resp = ui.button(wid_n("addons.row", i as u64), hit);
    if resp.hovered || resp.pressed {
        super::card::fill_box(ui, card, hit, 10.0, Color::white(if resp.pressed { 0.10 } else { 0.06 }));
    }
    let cy = r.center().y;

    // The badge scales with the row, centred in a fixed column so the text
    // never shifts. Only an add-on that is on wears the accent.
    let badge = if one_line {
        BADGE_ONE_LINE.min(r.h - 2.0)
    } else {
        (r.h * 0.78).round().min(BADGE_MAX)
    };
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
    // The size (or Pro), while there is a choice to make.
    let mut text_right = sw.x - 16.0;
    if status == Status::Off {
        let size = if a.pro {
            "Pro".to_string()
        } else if a.downloads() {
            fmt_bytes(a.size())
        } else {
            "Built in".to_string()
        };
        let style = if a.pro {
            TextStyle::footnote().weight(Weight::Semibold).color(crate::oyen::ORANGE).right()
        } else {
            TextStyle::footnote().color(palette::TEXT_DIM).right()
        };
        let w = ui.c.measure(&size, &style, 200.0).0.ceil();
        ui.c.text(&size, Rect::new(sw.x - 10.0 - w, r.y, w, r.h), &style);
        text_right = sw.x - 10.0 - w - 16.0;
    }

    let x = r.x + TEXT_X;
    let w = (text_right - x).max(0.0);
    let progress = status.progress();
    let line = status.line(a, MAC);
    let line_color = if status == Status::Failed { palette::ORANGE } else { palette::TEXT_DIM };
    let head = TextStyle::headline();
    let foot = TextStyle::footnote().color(line_color);
    // Lifted a little while a bar is shown along the bottom edge.
    let lift = if progress.is_some() { 2.0 } else { 0.0 };

    if one_line {
        // The name, then the line after it, on the row's middle.
        let ty = (r.y + ((r.h - TITLE_H) / 2.0).floor() - lift).max(r.y);
        let tw = ui.c.measure(a.title, &head, w).0.ceil().min(w);
        ui.c.text(a.title, Rect::new(x, ty, tw, TITLE_H), &head);
        let lx = x + tw + LINE_GAP;
        ui.c.text(&line, Rect::new(lx, ty + 2.0, (x + w - lx).max(0.0), LINE_H), &foot);
    } else {
        // The name above, the line below, centred together.
        let top = r.y + ((r.h - (TITLE_H + LINE_H)) / 2.0).floor() - lift;
        ui.c.text(a.title, Rect::new(x, top, w, TITLE_H), &head);
        ui.c.text(&line, Rect::new(x, top + TITLE_H, w, LINE_H), &foot);
    }

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
    use crate::state::OyenzaLink;
    use poni_core::settings::Settings;
    use poni_core::store::Store;

    #[test]
    fn every_row_offered_fits_the_page() {
        let mut st = AppState::new(Settings::default(), Store::default());
        let without = rows(&st).len();
        // Oyenza Studio Recorder installed, and signed in with Pro where the
        // Downloader is offered: the recorder, IELTS and the Downloader join.
        st.recorder = true;
        st.store.recorder_island = true;
        st.downloader_offered = true;
        st.oyenza.link = OyenzaLink::Connected { account: "a•••@example.com".into() };
        st.oyenza.pro = Some(true);
        assert_eq!(rows(&st).len(), without + 3);
        assert!(height(&st) <= MAX_H, "the window never grows for this page");
        let h = row_h(rows(&st).len());
        assert!(h < TWO_LINES, "that many rows are a line each");
        assert!(BADGE_ONE_LINE.min(h - 2.0) + 2.0 <= h, "{h} DIPs: the badges keep apart");
    }

    #[test]
    fn rows_stay_within_their_bounds() {
        for n in 1..=14 {
            let h = row_h(n);
            assert!((ROW_MIN..=ROW_MAX).contains(&h), "n = {n}, h = {h}");
        }
        // Up to the seven that fit as two lines each, the page keeps its height.
        for n in 1..=12 {
            assert!(ROWS_Y + n as f32 * row_h(n) <= MAX_H, "n = {n}");
        }
    }

    #[test]
    fn the_downloader_is_listed_and_switches_only_with_studio_pro() {
        use poni_core::addons::{DOWNLOADER, NOTIFICATIONS};
        let mut st = AppState::new(Settings::default(), Store::default());
        st.downloader_offered = true;
        assert!(!pro_wanted(&st), "nothing to ask about at sign-in");
        // The Downloader switched on before, its tools in place.
        st.settings.set_addon(DOWNLOADER.id, true);
        st.addons.insert(DOWNLOADER.id, Status::Ready);
        assert!(pro_wanted(&st), "asked at sign-in, though not listed yet");
        let listed = |st: &AppState| rows(st).iter().any(|a| a.id == DOWNLOADER.id);
        for (link, pro) in [
            (OyenzaLink::Off, None),
            (OyenzaLink::Connected { account: "a•••@example.com".into() }, None),
            (OyenzaLink::Connected { account: "a•••@example.com".into() }, Some(false)),
        ] {
            st.oyenza.link = link;
            st.oyenza.pro = pro;
            assert!(!listed(&st), "not listed without Pro ({pro:?})");
            assert_eq!(status(&st, &DOWNLOADER), Status::Off, "stays off");
        }
        st.oyenza.pro = Some(true);
        assert!(listed(&st));
        assert_eq!(status(&st, &DOWNLOADER), Status::Ready);
        // Not where it isn't offered (the Store build, the Mac), Pro or not.
        st.downloader_offered = false;
        assert!(!listed(&st) && !pro_wanted(&st));
        // Notifications is free (Luca, 28 Oct): listed and on without Pro.
        st.oyenza = Default::default();
        st.settings.set_addon(NOTIFICATIONS.id, true);
        st.addons.insert(NOTIFICATIONS.id, Status::Ready);
        assert_eq!(status(&st, &NOTIFICATIONS), Status::Ready);
        assert_eq!(rows(&st).iter().any(|a| a.id == NOTIFICATIONS.id), cfg!(windows));
    }
}