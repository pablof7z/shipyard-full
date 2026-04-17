pub(crate) const DEFAULT_BASE_BACKOFF_SECONDS: i64 = 30;
const MAX_BACKOFF_SECONDS: i64 = 24 * 60 * 60;

pub(crate) fn configured_base_backoff_seconds() -> i64 {
    let value = std::env::var("SHIPYARD_WORKER_BASE_BACKOFF_SECONDS").ok();
    parse_base_backoff_seconds(value.as_deref())
}

fn parse_base_backoff_seconds(value: Option<&str>) -> i64 {
    value
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|seconds| *seconds > 0)
        .unwrap_or(DEFAULT_BASE_BACKOFF_SECONDS)
}

pub(crate) fn retry_backoff_seconds(attempts: i32, base_seconds: i64) -> i64 {
    let base_seconds = base_seconds.max(1);
    let exponent = attempts.max(1).saturating_sub(1) as u32;
    let multiplier = 2_i64.saturating_pow(exponent);

    base_seconds
        .saturating_mul(multiplier)
        .min(MAX_BACKOFF_SECONDS)
}

pub(crate) fn retries_exhausted(attempts: i32, max_attempts: i32) -> bool {
    attempts >= max_attempts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_backoff_is_exponential() {
        assert_eq!(retry_backoff_seconds(0, 10), 10);
        assert_eq!(retry_backoff_seconds(1, 10), 10);
        assert_eq!(retry_backoff_seconds(2, 10), 20);
        assert_eq!(retry_backoff_seconds(3, 10), 40);
    }

    #[test]
    fn base_backoff_ignores_invalid_env_values() {
        assert_eq!(parse_base_backoff_seconds(Some("45")), 45);
        assert_eq!(
            parse_base_backoff_seconds(Some("0")),
            DEFAULT_BASE_BACKOFF_SECONDS
        );
        assert_eq!(
            parse_base_backoff_seconds(Some("nope")),
            DEFAULT_BASE_BACKOFF_SECONDS
        );
        assert_eq!(
            parse_base_backoff_seconds(None),
            DEFAULT_BASE_BACKOFF_SECONDS
        );
    }

    #[test]
    fn retries_are_exhausted_at_max_attempts() {
        assert!(!retries_exhausted(1, 3));
        assert!(!retries_exhausted(2, 3));
        assert!(retries_exhausted(3, 3));
        assert!(retries_exhausted(4, 3));
    }
}
