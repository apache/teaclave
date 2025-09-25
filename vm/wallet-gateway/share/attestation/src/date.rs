// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::fmt;

/// Represents a date and time.
///
/// This structure stores date and time components as individual fields,
/// providing a simple way to work with temporal data in attestation contexts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DateTime {
    /// Seconds after the minute - [0, 59]
    pub sec: i32,
    /// Minutes after the hour - [0, 59]
    pub min: i32,
    /// Hours after midnight - [0, 23]
    pub hour: i32,
    /// Day of the month - [1, 31]
    pub day: i32,
    /// Months since January - [1, 12]
    pub month: i32,
    /// Years (full year, e.g., 2025)
    pub year: i32,
}

impl DateTime {
    /// Creates a new DateTime instance with default values (all zeros).
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a DateTime from epoch seconds.
    ///
    /// # Arguments
    /// * `timestamp` - Unix timestamp in seconds since January 1, 1970
    ///
    /// # Returns
    /// A new DateTime instance representing the given timestamp
    pub fn from_timestamp(timestamp: i64) -> Self {
        let mut dt = Self::new();
        seconds_to_datetime(timestamp, &mut dt);
        dt
    }

    /// Extracts the date component from this DateTime.
    pub fn date(&self) -> Date {
        Date {
            day: self.day,
            month: self.month,
            year: self.year,
        }
    }

    /// Validates that all fields are within their expected ranges.
    pub fn is_valid(&self) -> bool {
        self.sec >= 0
            && self.sec <= 59
            && self.min >= 0
            && self.min <= 59
            && self.hour >= 0
            && self.hour <= 23
            && self.day >= 1
            && self.day <= 31
            && self.month >= 1
            && self.month <= 12
            && self.year >= 1970 // Assuming epoch-based timestamps
    }

    /// Returns the total seconds since midnight.
    pub fn seconds_since_midnight(&self) -> i32 {
        self.hour * 3600 + self.min * 60 + self.sec
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:02}{:02}{:02}{:02}{:02}{:02}Z",
            self.year % 100,
            self.month,
            self.day,
            self.hour,
            self.min,
            self.sec
        )
    }
}

/// Represents a date without time information.
///
/// This structure stores only the date components (year, month, day),
/// useful when time information is not needed or relevant.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Date {
    /// Day of the month - [1, 31]
    pub day: i32,
    /// Months since January - [1, 12]
    pub month: i32,
    /// Years (full year, e.g., 2025)
    pub year: i32,
}

impl Date {
    /// Creates a new Date instance with default values (all zeros).
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a Date with specific year, month, and day values.
    ///
    /// # Arguments
    /// * `year` - The year (e.g., 2025)
    /// * `month` - The month [1, 12]
    /// * `day` - The day of the month [1, 31]
    pub fn from_ymd(year: i32, month: i32, day: i32) -> Self {
        Self { year, month, day }
    }

    /// Validates that all fields are within their expected ranges.
    ///
    /// Note: This performs basic range validation but doesn't check
    /// for month-specific day limits (e.g., February 30th would pass).
    pub fn is_valid(&self) -> bool {
        self.day >= 1 && self.day <= 31 && self.month >= 1 && self.month <= 12 && self.year >= 1970
        // Assuming epoch-based timestamps
    }

    /// Checks if the year is a leap year.
    pub fn is_leap_year(&self) -> bool {
        is_leap_year(self.year as i64)
    }

    /// Returns the number of days in the current month.
    pub fn days_in_month(&self) -> i32 {
        get_days_in_month(self.year as i64, self.month as i64) as i32
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02} UTC", self.year, self.month, self.day)
    }
}

/// Convert epoch seconds into date time.
///
/// Converts a Unix timestamp (seconds since January 1, 1970 00:00:00 UTC)
/// into a DateTime structure. This function handles leap years correctly
/// and updates the provided DateTime instance in-place.
///
/// # Arguments
/// * `ts` - Unix timestamp in seconds
/// * `tm` - Mutable reference to DateTime structure to be updated
///
/// # Note
/// This function assumes the input timestamp is valid (>= 0) and
/// represents a time after the Unix epoch.
pub fn seconds_to_datetime(ts: i64, tm: &mut DateTime) {
    // Constants
    const SECONDS_PER_MINUTE: i64 = 60;
    const SECONDS_PER_HOUR: i64 = 3600;
    const SECONDS_PER_DAY: i64 = 86400;

    // Calculate time components from seconds within the day
    let day_seconds = ts % SECONDS_PER_DAY;
    let mut day_number = ts / SECONDS_PER_DAY;

    tm.sec = (day_seconds % SECONDS_PER_MINUTE) as i32;
    tm.min = ((day_seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE) as i32;
    tm.hour = (day_seconds / SECONDS_PER_HOUR) as i32;

    // Calculate year
    let mut year = 1970i64;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if day_number >= days_in_year {
            day_number -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }
    tm.year = year as i32;

    // Calculate month and day
    let mut month = 1;
    while month <= 12 {
        let days_in_month = get_days_in_month(year, month);
        if day_number >= days_in_month {
            day_number -= days_in_month;
            month += 1;
        } else {
            break;
        }
    }
    tm.month = month as i32;
    tm.day = (day_number + 1) as i32; // Convert from 0-based to 1-based day
}

/// Helper function to determine if a year is a leap year.
///
/// A year is a leap year if:
/// - It's divisible by 4, AND
/// - If it's divisible by 100, it must also be divisible by 400
fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Helper function to get the number of days in a specific month of a year.
///
/// # Arguments
/// * `year` - The year (used to determine if it's a leap year for February)
/// * `month` - The month [1, 12]
///
/// # Returns
/// Number of days in the specified month
fn get_days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0, // Invalid month
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime_from_timestamp() {
        // Test Unix epoch
        let dt = DateTime::from_timestamp(0);
        assert_eq!(dt.year, 1970);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 1);
        assert_eq!(dt.hour, 0);
        assert_eq!(dt.min, 0);
        assert_eq!(dt.sec, 0);

        // Test timestamp for January 1, 1971 00:00:00 UTC (365 days = 31536000 seconds)
        let dt = DateTime::from_timestamp(31536000);
        assert_eq!(dt.year, 1971);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 1);
        assert_eq!(dt.hour, 0);
        assert_eq!(dt.min, 0);
        assert_eq!(dt.sec, 0);

        // Test timestamp for January 2, 1970 12:30:45 UTC
        let dt = DateTime::from_timestamp(86400 + 12 * 3600 + 30 * 60 + 45); // 131445
        assert_eq!(dt.year, 1970);
        assert_eq!(dt.month, 1);
        assert_eq!(dt.day, 2);
        assert_eq!(dt.hour, 12);
        assert_eq!(dt.min, 30);
        assert_eq!(dt.sec, 45);
    }

    #[test]
    fn test_leap_year() {
        assert!(is_leap_year(2000)); // Divisible by 400
        assert!(is_leap_year(2004)); // Divisible by 4
        assert!(!is_leap_year(1900)); // Divisible by 100 but not 400
        assert!(!is_leap_year(2001)); // Not divisible by 4
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(get_days_in_month(2023, 1), 31); // January
        assert_eq!(get_days_in_month(2023, 2), 28); // February (non-leap)
        assert_eq!(get_days_in_month(2024, 2), 29); // February (leap)
        assert_eq!(get_days_in_month(2023, 4), 30); // April
    }

    #[test]
    fn test_datetime_validation() {
        let valid_dt = DateTime::from_timestamp(1262391174);
        assert!(valid_dt.is_valid());

        let mut invalid_dt = DateTime::new();
        invalid_dt.sec = 70; // Invalid seconds
        assert!(!invalid_dt.is_valid());
    }

    #[test]
    fn test_date_validation() {
        let valid_date = Date::from_ymd(2023, 6, 15);
        assert!(valid_date.is_valid());

        let invalid_date = Date::from_ymd(2023, 13, 15); // Invalid month
        assert!(!invalid_date.is_valid());
    }
}
