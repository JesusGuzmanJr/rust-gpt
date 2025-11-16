use {
    crate::internationalization::Internationalization,
    chrono::{DateTime, Datelike, Timelike, Utc},
    icu::{
        datetime::input::{Date, Time},
        time::Nanosecond,
    },
};

/// Formats a UTC datetime in a human-readable format according to the user's
/// locale and timezone.
///
/// This function takes a UTC datetime (as stored in the database) and converts
/// it to the user's local timezone before formatting it according to their
/// locale preferences.
///
/// # Example
/// ```ignore
/// use chrono::Utc;
/// use chrono_tz::America::New_York;
/// use icu::locale::locale;
///
/// let utc_time = Utc::now();
/// let locale = locale!("en-US");
/// let formatted = today_implicit_human_readable(&utc_time, &locale, &New_York);
/// // Result: "Sun, Oct 19, 2025, 10:30 AM" (for a day that's not today)
/// // Result: "3:30 PM" (for a day that is today)
/// ```
pub(crate) fn today_implied_readable_datetime(
    datetime: &DateTime<Utc>,
    Internationalization { locale, timezone }: &Internationalization,
) -> String {
    let local_datetime = datetime.with_timezone(timezone);

    let time = Time::try_new(
        local_datetime.hour() as u8,
        local_datetime.minute() as u8,
        local_datetime.second() as u8,
        local_datetime.nanosecond(),
    )
    .expect("failed to create Time");

    if time <= Time::try_new(23, 59, 59, 999_999_999).expect("failed to create time") {
        icu::datetime::DateTimeFormatter::try_new(
            locale.into(),
            icu::datetime::fieldsets::T::medium()
                .with_time_precision(icu::datetime::options::TimePrecision::Minute),
        )
        .expect("failed to create DateTimeFormatter")
        .format(&time)
        .to_string()
    } else {
        let date = Date::try_new_iso(
            local_datetime.year(),
            local_datetime.month() as u8,
            local_datetime.day() as u8,
        )
        .expect("failed to create Date");

        icu::datetime::DateTimeFormatter::try_new(
            locale.into(),
            icu::datetime::fieldsets::YMD::medium(),
        )
        .expect("failed to create DateTimeFormatter")
        .format(&icu::datetime::input::DateTime { date, time })
        .to_string()
    }
}
