//! Minimal date/time parsing for the Dropbox and WebDAV clients.
//!
//! Both clients return modification times as strings (ISO 8601 UTC for
//! Dropbox, RFC 1123 for WebDAV) but the app models them as seconds
//! since the Unix epoch (`u64`). To avoid pulling in a full date crate
//! (the project is dependency-light), we parse these two fixed formats
//! by hand and convert civil dates to epoch seconds with Howard
//! Hinnant's `days_from_civil` algorithm.

/// Days since 1970-01-01 for a civil date (Howard Hinnant's algorithm).
fn days_from_civil(mut y: i64, m: u32, d: u32) -> i64 {
    // Treat Jan/Feb as months 13/14 of the previous year.
    y -= i64::from(m <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u32; // [0, 399]
    let doy = (153 * (m as i64 + if m > 2 { -3 } else { 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe as i64 * 365 + yoe as i64 / 4 - yoe as i64 / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `(year, month, day, hour, minute, second)` → seconds since the Unix
/// epoch (UTC). Negative results are clamped to 0.
pub fn ymdhms_to_epoch(y: i64, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> u64 {
    let days = days_from_civil(y, mo, d);
    let secs = days * 86_400 + h as i64 * 3_600 + mi as i64 * 60 + s as i64;
    secs.max(0) as u64
}

/// Parse a Dropbox `server_modified` timestamp (ISO 8601 UTC):
/// `2015-05-12T15:50:38Z` (optionally with fractional seconds).
pub fn parse_iso8601_utc(s: &str) -> Option<u64> {
    let s = s.trim();
    let (date, time) = s.split_once('T')?;

    let mut dp = date.split('-');
    let y: i64 = dp.next()?.parse().ok()?;
    let mo: u32 = dp.next()?.parse().ok()?;
    let d: u32 = dp.next()?.parse().ok()?;

    // Dropbox always returns UTC ("Z"); tolerate lowercase "z" too.
    let time = time.strip_suffix('Z').or_else(|| time.strip_suffix('z'))?;
    // Drop fractional seconds (".123456").
    let time = time.split('.').next().unwrap_or(time);
    let mut tp = time.split(':');
    let h: u32 = tp.next()?.parse().ok()?;
    let mi: u32 = tp.next()?.parse().ok()?;
    let s: u32 = tp.next()?.parse().ok()?;

    Some(ymdhms_to_epoch(y, mo, d, h, mi, s))
}

/// Month name (English) → 1-based month number.
fn month_number(s: &str) -> Option<u32> {
    match s.to_ascii_lowercase().as_str() {
        "jan" => Some(1),
        "feb" => Some(2),
        "mar" => Some(3),
        "apr" => Some(4),
        "may" => Some(5),
        "jun" => Some(6),
        "jul" => Some(7),
        "aug" => Some(8),
        "sep" => Some(9),
        "oct" => Some(10),
        "nov" => Some(11),
        "dec" => Some(12),
        _ => None,
    }
}

/// Parse a WebDAV `getlastmodified` timestamp (RFC 1123):
/// `Mon, 15 Aug 2005 15:52:01 GMT` (some servers omit the weekday).
pub fn parse_rfc1123(s: &str) -> Option<u64> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    // Forma con giorno della settimana: `Mon, 15 Aug 2005 15:52:01 GMT`.
    // Forma senza:                    `15 Aug 2005 15:52:01 GMT`.
    let (day, mon, year, time) = if parts.first()?.ends_with(',') {
        (
            *parts.get(1)?,
            *parts.get(2)?,
            *parts.get(3)?,
            *parts.get(4)?,
        )
    } else {
        (
            *parts.first()?,
            *parts.get(1)?,
            *parts.get(2)?,
            *parts.get(3)?,
        )
    };

    let d: u32 = day.trim_end_matches(',').parse().ok()?;
    let mo = month_number(mon)?;
    let y: i64 = year.parse().ok()?;

    let mut tp = time.split(':');
    let h: u32 = tp.next()?.parse().ok()?;
    let mi: u32 = tp.next()?.parse().ok()?;
    let s: u32 = tp.next()?.parse().ok()?;

    Some(ymdhms_to_epoch(y, mo, d, h, mi, s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iso8601_utc_parses() {
        // 2015-05-12T15:50:38Z == 1431445838.
        assert_eq!(parse_iso8601_utc("2015-05-12T15:50:38Z"), Some(1431445838));
    }

    #[test]
    fn iso8601_utc_parses_fractional() {
        assert_eq!(
            parse_iso8601_utc("2015-05-12T15:50:38.000Z"),
            Some(1431445838)
        );
    }

    #[test]
    fn rfc1123_parses_with_weekday() {
        // Mon, 15 Aug 2005 15:52:01 GMT == 1124121121.
        assert_eq!(
            parse_rfc1123("Mon, 15 Aug 2005 15:52:01 GMT"),
            Some(1124121121)
        );
    }

    #[test]
    fn rfc1123_parses_without_weekday() {
        assert_eq!(parse_rfc1123("15 Aug 2005 15:52:01 GMT"), Some(1124121121));
    }
}
