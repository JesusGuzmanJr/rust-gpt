use chrono::{DateTime, Utc};

/// Words used to measure time like seconds, days, years, etc.
enum TemporalUnit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

/// The threshold for the `just now` in seconds.
const JUST_NOW_THRESHOLD_SECONDS: i64 = 15;

/// Formats the datetime as a `x seconds/minutes/hours/weeks/months/years ago`.
pub(crate) fn ago(datetime: &DateTime<Utc>) -> String {
    let seconds = (Utc::now() - *datetime).num_seconds();

    if seconds < JUST_NOW_THRESHOLD_SECONDS {
        "Just now".into()
    } else if seconds < 60 {
        ago_phrase(seconds, TemporalUnit::Second)
    } else if seconds < 60 * 60 {
        ago_phrase(seconds / 60, TemporalUnit::Minute)
    } else if seconds < 60 * 60 * 24 {
        ago_phrase(seconds / 60 / 60, TemporalUnit::Hour)
    } else if seconds < 60 * 60 * 24 * 7 {
        ago_phrase(seconds / 60 / 60 / 24, TemporalUnit::Day)
    } else if seconds < 60 * 60 * 24 * 7 * 4 {
        ago_phrase(seconds / 60 / 60 / 24 / 7, TemporalUnit::Week)
    } else if seconds < 60 * 60 * 24 * 7 * 4 * 12 {
        let mut value = (seconds as f64 / 60_f64 / 60_f64 / 24_f64 / 7_f64 / 4.348) as _;
        if value == 0 {
            value = 1;
        }
        ago_phrase(value, TemporalUnit::Month)
    } else {
        let mut value = (seconds as f64 / 60_f64 / 60_f64 / 24_f64 / 7_f64 / 4.348 / 12_f64) as _;
        if value == 0 {
            value = 1;
        }
        ago_phrase(value, TemporalUnit::Year)
    }
}

/// Grammatical number.
enum Number {
    Singular,
    Plural,
}

/// Returns the word for the unit in the provided language.
fn unit_word(unit: TemporalUnit, number: Number) -> &'static str {
    match unit {
        TemporalUnit::Second => match number {
            Number::Singular => "second",
            Number::Plural => "seconds",
        },
        TemporalUnit::Minute => match number {
            Number::Singular => "minute",
            Number::Plural => "minutes",
        },
        TemporalUnit::Hour => match number {
            Number::Singular => "hour",
            Number::Plural => "hours",
        },
        TemporalUnit::Day => match number {
            Number::Singular => "day",
            Number::Plural => "days",
        },
        TemporalUnit::Week => match number {
            Number::Singular => "week",
            Number::Plural => "weeks",
        },
        TemporalUnit::Month => match number {
            Number::Singular => "month",
            Number::Plural => "months",
        },
        TemporalUnit::Year => match number {
            Number::Singular => "year",
            Number::Plural => "years",
        },
    }
}

/// Formats the datetime as a `x seconds ago` in the provided language.
fn ago_phrase(value: i64, unit: TemporalUnit) -> String {
    match value {
        1 => format!("1 {} ago", unit_word(unit, Number::Singular)),
        _ => format!("{value} {} ago", unit_word(unit, Number::Plural)),
    }
}

#[cfg(test)]
mod test {
    use {super::*, chrono::Duration};

    #[test]
    fn test_10_seconds_ago() {
        let datetime = Utc::now() - Duration::seconds(10);
        assert_eq!(ago(&datetime), "just now");
    }

    #[test]
    fn test_30_seconds_ago() {
        let datetime = Utc::now() - Duration::seconds(30);
        assert_eq!(ago(&datetime), "30 seconds ago");
    }

    #[test]
    fn test_1_day_ago() {
        let jitter = Duration::seconds(7);
        let datetime = Utc::now() - Duration::days(1) - jitter;
        assert_eq!(ago(&datetime), "1 day ago");
    }

    #[test]
    fn test_6_day_ago() {
        let jitter = Duration::hours(23);
        let datetime = Utc::now() - Duration::days(6) - jitter;
        assert_eq!(ago(&datetime), "6 days ago");
    }

    #[test]
    fn test_1_week_ago() {
        let jitter = Duration::days(4);
        let datetime = Utc::now() - Duration::weeks(1) - jitter;
        assert_eq!(ago(&datetime), "1 week ago");
    }

    #[test]
    fn test_1_months_ago() {
        let jitter = Duration::hours(1);
        let datetime = Utc::now() - Duration::hours(30 * 24) - jitter;
        assert_eq!(ago(&datetime), "1 month ago");
    }

    #[test]
    fn test_6_months_ago() {
        let jitter = Duration::hours(1);
        let datetime = Utc::now() - Duration::hours((30.437_f64 * 24_f64 * 6_f64) as _) - jitter;
        assert_eq!(ago(&datetime), "6 months ago");
    }
}
