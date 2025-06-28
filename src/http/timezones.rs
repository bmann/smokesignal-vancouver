use anyhow::{anyhow, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use itertools::Itertools;

use crate::storage::handle::model::Handle;

pub fn supported_timezones(handle: Option<&Handle>) -> (&str, Vec<&str>) {
    let handle_tz = handle
        .and_then(|handle| handle.tz.parse().ok())
        .unwrap_or(chrono_tz::UTC);

    let timezones = vec![
        chrono_tz::America::Anchorage.name(),
        chrono_tz::America::Chicago.name(),
        chrono_tz::America::Denver.name(),
        chrono_tz::America::Los_Angeles.name(),
        chrono_tz::America::New_York.name(),
        chrono_tz::America::Phoenix.name(),
        chrono_tz::America::Puerto_Rico.name(),
        chrono_tz::Australia::Darwin.name(),
        chrono_tz::Australia::Perth.name(),
        chrono_tz::Australia::Sydney.name(),
        chrono_tz::Canada::Atlantic.name(),
        chrono_tz::Canada::Newfoundland.name(),
        chrono_tz::CET.name(),
        chrono_tz::EET.name(),
        chrono_tz::Europe::London.name(),
        chrono_tz::GMT.name(),
        chrono_tz::Pacific::Auckland.name(),
        chrono_tz::Pacific::Chatham.name(),
        chrono_tz::Pacific::Guam.name(),
        chrono_tz::Pacific::Honolulu.name(),
        chrono_tz::US::Alaska.name(),
        chrono_tz::US::Aleutian.name(),
        chrono_tz::US::Samoa.name(),
        chrono_tz::WET.name(),
        handle_tz.name(),
    ];

    (
        handle_tz.name(),
        timezones.into_iter().sorted().dedup().collect(),
    )
}

/// Combines an HTML date input value and HTML time input value into a single
/// UTC datetime, using the provided timezone.
///
/// # Arguments
///
/// * `date_str` - A string in the format "YYYY-MM-DD" from an HTML date input
/// * `time_str` - A string in the format "HH:MM" from an HTML time input
/// * `timezone` - A string representing a timezone (e.g., "America/New_York")
///
/// # Returns
///
/// * `Result<DateTime<Utc>>` - The combined datetime in UTC, or an error if parsing fails
///
/// # Example
///
/// ```no_run
/// # use anyhow::Result;
/// # use smokesignal::http::timezones::combine_html_datetime;
/// # fn example() -> Result<()> {
/// let date_str = "2025-05-06";
/// let time_str = "18:00";
/// let timezone = "America/New_York".parse::<chrono_tz::Tz>().unwrap();
/// let utc_datetime = combine_html_datetime(date_str, time_str, timezone)?;
/// # Ok(())
/// # }
/// ```
pub fn combine_html_datetime(
    date_str: &str,
    time_str: &str,
    timezone: chrono_tz::Tz,
) -> Result<DateTime<Utc>> {
    // Combine date and time strings
    let datetime_str = format!("{}T{}", date_str, time_str);

    // Parse the combined string into a NaiveDateTime
    let naive_dt = NaiveDateTime::parse_from_str(&datetime_str, "%Y-%m-%dT%H:%M")
        .map_err(|e| anyhow!("Failed to parse date and time: {}", e))?;

    // Convert to timezone-aware datetime in the specified timezone
    let local_dt = naive_dt
        .and_local_timezone(timezone)
        .single()
        .ok_or_else(|| anyhow!("Ambiguous or non-existent local time"))?;

    // Convert to UTC
    Ok(local_dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_combine_html_datetime() {
        // Test a simple case with New York timezone (Eastern Time)
        let tz = "America/New_York".parse::<chrono_tz::Tz>().unwrap();

        let result = combine_html_datetime("2025-05-06", "18:00", tz).unwrap();

        // Expected: 6:00 PM Eastern = 10:00 PM UTC (during daylight saving time)
        let expected = Utc.with_ymd_and_hms(2025, 5, 6, 22, 0, 0).unwrap();
        assert_eq!(result, expected);

        // Test with UTC timezone
        let tz = "UTC".parse::<chrono_tz::Tz>().unwrap();
        let result = combine_html_datetime("2025-05-06", "18:00", tz).unwrap();
        let expected = Utc.with_ymd_and_hms(2025, 5, 6, 18, 0, 0).unwrap();
        assert_eq!(result, expected);

        // Test with Tokyo timezone (no daylight saving time)
        let tz = "Asia/Tokyo".parse::<chrono_tz::Tz>().unwrap();
        let result = combine_html_datetime("2025-05-06", "18:00", tz).unwrap();
        let expected = Utc.with_ymd_and_hms(2025, 5, 6, 9, 0, 0).unwrap();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_combine_html_datetime_invalid_inputs() {
        let tz = "America/New_York".parse::<chrono_tz::Tz>().unwrap();

        // Test with invalid date format
        let result = combine_html_datetime("05/06/2025", "18:00", tz);
        assert!(result.is_err());

        // Test with invalid time format
        let result = combine_html_datetime("2025-05-06", "6:00 PM", tz);
        assert!(result.is_err());
    }

    #[test]
    fn test_combine_html_datetime_edge_cases() {
        // Test with daylight saving time transition
        // March 8, 2026, 2:30 AM in US/Pacific - Should be during DST transition
        let tz = "US/Pacific".parse::<chrono_tz::Tz>().unwrap();
        let result = combine_html_datetime("2026-03-08", "02:30", tz);
        assert!(result.is_err());

        // Midnight
        let tz = "UTC".parse::<chrono_tz::Tz>().unwrap();
        let result = combine_html_datetime("2025-05-06", "00:00", tz).unwrap();
        let expected = Utc.with_ymd_and_hms(2025, 5, 6, 0, 0, 0).unwrap();
        assert_eq!(result, expected);
    }
}
