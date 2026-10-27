//! Workout reminders from the user's local reminder API.

use crate::commands::Command;
use crate::gfx::{TextStyle, Weight};
use crate::icons::Icon;
use crate::state::AppState;
use crate::ui::{wid, PillStyle, Ui, PILL_SMALL};
use poni_core::datetime::{fmt_clock, fmt_date_short};
use poni_core::geom::{palette, Rect};

const HEIGHT: f32 = 188.0;
const ROW_H: f32 = 34.0;

pub fn height(_st: &AppState) -> f32 {
    HEIGHT
}

pub fn draw(ui: &mut Ui, st: &mut AppState, area: Rect) {
    let refresh_w = ui.pill_width("Refresh", true, true);
    let refresh = Rect::new(area.right() - refresh_w, area.y, refresh_w, PILL_SMALL);
    if ui
        .pill(
            wid("reminders.refresh"),
            refresh,
            "Refresh",
            Some(Icon::Reset),
            PillStyle::Utility,
        )
        .clicked
    {
        st.cmd(Command::RefreshReminders);
    }

    ui.c.text(
        &format!("Today · {}", fmt_date_short(&st.local)),
        Rect::new(
            area.x,
            area.y,
            (refresh.x - area.x - 10.0).max(0.0),
            PILL_SMALL,
        ),
        &TextStyle::headline(),
    );

    let results = poni_core::reminders::today(&st.reminders.items, st.unix, st.local);
    let top = area.y + PILL_SMALL + 10.0;
    if st.reminders.loading && st.reminders.items.is_empty() {
        ui.c.text(
            "Loading reminders…",
            Rect::new(area.x, top, area.w, 20.0),
            &TextStyle::body().color(palette::TEXT_DIM),
        );
        return;
    }
    if let Some(error) = &st.reminders.error {
        ui.c.text(
            "Couldn't load reminders",
            Rect::new(area.x, top, area.w, 20.0),
            &TextStyle::body()
                .weight(Weight::Semibold)
                .color(palette::ORANGE),
        );
        ui.c.text(
            error,
            Rect::new(area.x, top + 23.0, area.w, 38.0),
            &TextStyle::footnote().color(palette::TEXT_DIM),
        );
        return;
    }
    if results.is_empty() {
        let line = if st.reminders.loading {
            "Refreshing reminders…"
        } else {
            "No workouts scheduled for today."
        };
        ui.c.text(
            line,
            Rect::new(area.x, top, area.w, 20.0),
            &TextStyle::body().color(palette::TEXT_DIM),
        );
        return;
    }

    for (i, (reminder, local_time)) in results.iter().take(4).enumerate() {
        let y = top + i as f32 * ROW_H;
        let time = fmt_clock(local_time, st.settings.use_24h);
        let time_style = TextStyle::body().weight(Weight::Semibold).tabular();
        let (time_w, _) = ui.c.measure(&time, &time_style, 100.0);
        ui.c.text(&time, Rect::new(area.x, y, time_w + 4.0, 22.0), &time_style);
        ui.c.text(
            &reminder.title,
            Rect::new(
                area.x + time_w + 14.0,
                y,
                (area.w - time_w - 14.0).max(0.0),
                22.0,
            ),
            &TextStyle::body(),
        );
    }
    if st.reminders.loading {
        ui.c.text(
            "Refreshing…",
            Rect::new(area.x, area.bottom() - 17.0, area.w, 15.0),
            &TextStyle::caption().color(palette::TEXT_FAINT),
        );
    } else if results.len() > 4 {
        ui.c.text(
            &format!("{} more reminders today", results.len() - 4),
            Rect::new(area.x, area.bottom() - 17.0, area.w, 15.0),
            &TextStyle::caption().color(palette::TEXT_FAINT),
        );
    }
}
