//! Exact RFC 3339 to Unix-microsecond conversion. The store compares only integer times.
use crate::store::StoreError;

pub(crate) fn parse_rfc3339_us(s: &str) -> Result<i64, StoreError> {
    let b = s.as_bytes();
    if b.len() < 20
        || b.get(4) != Some(&b'-')
        || b.get(7) != Some(&b'-')
        || !matches!(b.get(10), Some(b'T' | b't'))
        || b.get(13) != Some(&b':')
        || b.get(16) != Some(&b':')
    {
        return Err(StoreError::Invalid(format!(
            "invalid RFC3339 timestamp: {s}"
        )));
    }
    let year = digits(b, 0, 4)? as i64;
    let month = digits(b, 5, 7)? as i64;
    let day = digits(b, 8, 10)? as i64;
    let hour = digits(b, 11, 13)? as i64;
    let minute = digits(b, 14, 16)? as i64;
    let second = digits(b, 17, 19)? as i64;
    if !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return Err(StoreError::Invalid(format!(
            "timestamp component out of range: {s}"
        )));
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    if day < 1 || day > month_days[(month - 1) as usize] {
        return Err(StoreError::Invalid(format!("day out of range: {s}")));
    }
    let mut i = 19;
    let mut micros = 0i64;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        let count = i - start;
        if count == 0 || count > 6 {
            return Err(StoreError::Invalid(
                "timestamp fraction must contain 1–6 digits".into(),
            ));
        }
        let mut fraction = digits(b, start, i)? as i64;
        for _ in count..6 {
            fraction *= 10;
        }
        micros = fraction;
    }
    let offset_seconds: i64 = match b.get(i) {
        Some(b'Z') | Some(b'z') if i + 1 == b.len() => 0,
        Some(sign @ (b'+' | b'-')) if i + 6 == b.len() && b.get(i + 3) == Some(&b':') => {
            let hh = digits(b, i + 1, i + 3)? as i64;
            let mm = digits(b, i + 4, i + 6)? as i64;
            if hh > 23 || mm > 59 {
                return Err(StoreError::Invalid("timezone offset out of range".into()));
            }
            let v = hh * 3600 + mm * 60;
            if *sign == b'+' { v } else { -v }
        }
        _ => {
            return Err(StoreError::Invalid(format!(
                "invalid RFC3339 timezone: {s}"
            )));
        }
    };
    let days = days_from_civil(year, month, day);
    let local_seconds = days
        .checked_mul(86_400)
        .and_then(|v| v.checked_add(hour * 3600 + minute * 60 + second))
        .ok_or_else(|| StoreError::Invalid("timestamp overflow".into()))?;
    let utc_seconds = local_seconds
        .checked_sub(offset_seconds)
        .ok_or_else(|| StoreError::Invalid("timestamp overflow".into()))?;
    utc_seconds
        .checked_mul(1_000_000)
        .and_then(|v| v.checked_add(micros))
        .ok_or_else(|| StoreError::Invalid("timestamp outside microsecond range".into()))
}

#[cfg(test)]
pub(crate) fn format_rfc3339_us(us: i64) -> String {
    let secs = us.div_euclid(1_000_000);
    let frac = us.rem_euclid(1_000_000);
    let days = secs.div_euclid(86_400);
    let daysec = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    let h = daysec / 3600;
    let min = (daysec % 3600) / 60;
    let sec = daysec % 60;
    if frac == 0 {
        format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{sec:02}Z")
    } else {
        let mut f = format!("{frac:06}");
        while f.ends_with('0') {
            f.pop();
        }
        format!("{y:04}-{m:02}-{d:02}T{h:02}:{min:02}:{sec:02}.{f}Z")
    }
}
fn digits(b: &[u8], start: usize, end: usize) -> Result<u32, StoreError> {
    let slice = b
        .get(start..end)
        .ok_or_else(|| StoreError::Invalid("truncated timestamp".into()))?;
    if !slice.iter().all(u8::is_ascii_digit) {
        return Err(StoreError::Invalid(
            "timestamp component is not numeric".into(),
        ));
    }
    Ok(slice.iter().fold(0u32, |n, c| n * 10 + (c - b'0') as u32))
}
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
#[cfg(test)]
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    if m <= 2 {
        y += 1;
    }
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rfc3339_round_trips_and_normalizes_offsets_with_integer_precision() {
        for (source, canonical) in [
            ("2026-11-15T00:00:00Z", "2026-11-15T00:00:00Z"),
            (
                "2026-11-15T05:30:00.123400+05:30",
                "2026-11-15T00:00:00.1234Z",
            ),
            ("1970-01-01T00:00:00.000001Z", "1970-01-01T00:00:00.000001Z"),
        ] {
            assert_eq!(
                format_rfc3339_us(parse_rfc3339_us(source).unwrap()),
                canonical
            );
        }
        assert!(parse_rfc3339_us("2026-11-15T00:00:00.1234567Z").is_err());
        assert!(parse_rfc3339_us("2026-02-29T00:00:00Z").is_err());
    }
}
