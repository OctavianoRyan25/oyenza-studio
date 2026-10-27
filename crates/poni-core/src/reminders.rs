//! Workout reminders returned by the user's reminder server.

use crate::datetime::{civil_from_days, days_from_civil, days_in_month, LocalDateTime};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Reminder {
    pub id: u64,
    pub title: String,
    #[serde(deserialize_with = "deserialize_timestamp")]
    pub scheduled_at: i64,
    pub status: String,
}

#[derive(Deserialize)]
struct Response {
    success: bool,
    data: Vec<Reminder>,
}

/// Read active reminders from the API's JSON response.
pub fn parse_response(json: &str) -> Result<Vec<Reminder>, String> {
    let response: Response =
        serde_json::from_str(json).map_err(|e| format!("invalid response: {e}"))?;
    if !response.success {
        return Err("server reported that the request failed".into());
    }
    Ok(response
        .data
        .into_iter()
        .filter(|r| r.status.eq_ignore_ascii_case("active"))
        .collect())
}

/// Return the reminders scheduled on the current local calendar day.
/// `now_local` supplies the system's current UTC offset through its wall clock.
pub fn today(
    items: &[Reminder],
    now_unix: i64,
    now_local: LocalDateTime,
) -> Vec<(Reminder, LocalDateTime)> {
    let today = days_from_civil(now_local.year, now_local.month, now_local.day);
    let local_now = today * 86_400
        + i64::from(now_local.hour) * 3_600
        + i64::from(now_local.minute) * 60
        + i64::from(now_local.second);
    let utc_offset = local_now - now_unix;
    items
        .iter()
        .filter_map(|item| {
            let local_unix = item.scheduled_at + utc_offset;
            let day = local_unix.div_euclid(86_400);
            (day == today).then(|| {
                let seconds = local_unix.rem_euclid(86_400);
                let (year, month, day) = civil_from_days(day);
                (
                    item.clone(),
                    LocalDateTime {
                        year,
                        month,
                        day,
                        hour: (seconds / 3_600) as u8,
                        minute: ((seconds / 60) % 60) as u8,
                        second: (seconds % 60) as u8,
                    },
                )
            })
        })
        .collect()
}

fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    parse_timestamp(&value)
        .ok_or_else(|| serde::de::Error::custom("expected an RFC 3339 timestamp"))
}

/// Parse RFC 3339 timestamps, including fractional seconds and numeric offsets.
pub fn parse_timestamp(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
    {
        return None;
    }
    let number = |range: std::ops::Range<usize>| -> Option<u32> {
        let part = bytes.get(range)?;
        part.iter().try_fold(0_u32, |n, b| {
            b.is_ascii_digit().then(|| n * 10 + u32::from(*b - b'0'))
        })
    };
    let year = number(0..4)? as i32;
    let month = number(5..7)? as u8;
    let day = number(8..10)? as u8;
    let hour = number(11..13)?;
    let minute = number(14..16)?;
    let second = number(17..19)?;
    if !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return None;
    }

    let mut at = 19;
    if bytes.get(at) == Some(&b'.') {
        at += 1;
        let start = at;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        if at == start {
            return None;
        }
    }
    let offset = match bytes.get(at).copied()? {
        b'Z' if at + 1 == bytes.len() => 0_i64,
        sign @ (b'+' | b'-') if at + 6 == bytes.len() && bytes.get(at + 3) == Some(&b':') => {
            let h = number(at + 1..at + 3)? as i64;
            let m = number(at + 4..at + 6)? as i64;
            if h > 23 || m > 59 {
                return None;
            }
            let offset = h * 3_600 + m * 60;
            if sign == b'+' {
                offset
            } else {
                -offset
            }
        }
        _ => return None,
    };

    let days = days_from_civil(year, month, day);
    Some(
        days * 86_400 + i64::from(hour) * 3_600 + i64::from(minute) * 60 + i64::from(second)
            - offset,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reminder(id: u64, scheduled_at: &str) -> Reminder {
        Reminder {
            id,
            title: format!("Workout {id}"),
            scheduled_at: parse_timestamp(scheduled_at).unwrap(),
            status: "active".into(),
        }
    }

    #[test]
    fn parses_response_and_keeps_only_active_reminders() {
        let items = parse_response(
            r#"{"success":true,"data":[{"id":1,"title":"Interval","scheduled_at":"2026-11-11T08:04:05Z","status":"active","created_at":"2026-10-27T06:07:45Z"},{"id":2,"title":"Done","scheduled_at":"2026-11-11T09:00:00Z","status":"done"}]}"#,
        )
        .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Interval");
        assert_eq!(items[0].scheduled_at, 1_794_384_245);
    }

    #[test]
    fn parses_fractional_and_offset_timestamps() {
        assert_eq!(
            parse_timestamp("2026-11-11T15:04:05.123+07:00"),
            parse_timestamp("2026-11-11T08:04:05Z")
        );
        assert_eq!(parse_timestamp("2026-11-11T08:04:05Z"), Some(1_794_384_245));
        assert_eq!(parse_timestamp("2026-02-30T08:04:05Z"), None);
        assert_eq!(parse_timestamp("2026-11-11T08:04:05"), None);
    }

    #[test]
    fn filters_by_local_calendar_day() {
        let items = [
            reminder(1, "2026-11-10T18:00:00Z"),
            reminder(2, "2026-11-11T06:00:00Z"),
            reminder(3, "2026-11-11T18:00:00Z"),
        ];
        let local = LocalDateTime {
            year: 2026,
            month: 11,
            day: 11,
            hour: 12,
            minute: 0,
            second: 0,
        };
        let day_start = days_from_civil(local.year, local.month, local.day) * 86_400;
        let found = today(&items, day_start + 5 * 3_600, local);
        assert_eq!(found.iter().map(|(r, _)| r.id).collect::<Vec<_>>(), [1, 2]);
        assert_eq!((found[0].1.hour, found[0].1.minute), (1, 0));
    }

    #[test]
    fn rejects_unsuccessful_server_response() {
        assert!(parse_response(r#"{"success":false,"data":[]}"#).is_err());
    }
}
