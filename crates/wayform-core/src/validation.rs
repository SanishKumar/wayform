pub(crate) const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

pub(crate) fn valid_time(value: i64) -> bool {
    // Four-digit UTC years, including year zero, in Unix milliseconds.
    (-62_167_219_200_000..=253_402_300_799_999).contains(&value)
}

pub(crate) fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(crate) fn timestamp(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 24 {
        return None;
    }
    for (index, expected) in [
        (4, b'-'),
        (7, b'-'),
        (10, b'T'),
        (13, b':'),
        (16, b':'),
        (19, b'.'),
        (23, b'Z'),
    ] {
        if bytes[index] != expected {
            return None;
        }
    }
    let number = |start: usize, end: usize| -> Option<i64> {
        bytes[start..end].iter().try_fold(0, |value, digit| {
            digit
                .is_ascii_digit()
                .then(|| value * 10 + i64::from(digit - b'0'))
        })
    };
    let year = number(0, 4)?;
    let month = number(5, 7)?;
    let day = number(8, 10)?;
    let hour = number(11, 13)?;
    let minute = number(14, 16)?;
    let second = number(17, 19)?;
    let millisecond = number(20, 23)?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let months = [
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
    if !(1..=12).contains(&month)
        || day < 1
        || day > months[(month - 1) as usize]
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    // Count Gregorian days from year 0000, including its leap day.
    let before_year =
        |year: i64| 365 * year + (year + 3) / 4 - (year + 99) / 100 + (year + 399) / 400;
    let days = before_year(year) - before_year(1970)
        + months[..(month - 1) as usize].iter().sum::<i64>()
        + day
        - 1;
    Some(((days * 24 + hour) * 60 + minute) * 60_000 + second * 1000 + millisecond)
}

#[cfg(test)]
mod tests {
    use super::timestamp;
    #[test]
    fn utc_format_calendar_and_extremes() {
        for (text, expected) in [
            ("0000-01-01T00:00:00.000Z", -62_167_219_200_000),
            ("1970-01-01T00:00:00.000Z", 0),
            ("1969-12-31T23:59:59.999Z", -1),
            ("2000-03-01T00:00:00.000Z", 951_868_800_000),
            ("9999-12-31T23:59:59.999Z", 253_402_300_799_999),
        ] {
            assert_eq!(timestamp(text), Some(expected), "{text}");
        }
        for text in [
            "1900-02-29T00:00:00.000Z",
            "2026-02-29T00:00:00.000Z",
            "2026-04-31T00:00:00.000Z",
            "2026-01-01T24:00:00.000Z",
            "2026-01-01T00:00:60.000Z",
            "2026-01-01T00:00:00Z",
            "2026-01-01T00:00:00.000z",
            "2026-01-01T00:00:00.000+00:00",
        ] {
            assert_eq!(timestamp(text), None, "{text}");
        }
        assert!(timestamp("0000-02-29T00:00:00.000Z").is_some());
        assert!(timestamp("2000-02-29T00:00:00.000Z").is_some());
    }
}
