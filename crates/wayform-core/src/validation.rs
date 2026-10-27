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
