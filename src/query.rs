use chrono::{Datelike, Duration, Local, Months, NaiveDate};

use crate::error::{WitError, Result};

/// Parsed result of a shorthand query
#[derive(Debug)]
pub enum Query {
    /// Show current weather for a location (live fetch)
    Current { location: String },
    /// Compare a location now vs a point in the past
    TimeDiff {
        location: String,
        past: TimeSpec,
    },
    /// Compare a location at two points in time
    TimeRange {
        location: String,
        from: TimeSpec,
        to: TimeSpec,
    },
    /// Compare two locations right now
    LocationDiff {
        left: String,
        right: String,
    },
    /// Compare two locations at a time in the past
    LocationDiffAt {
        left: String,
        right: String,
        time: TimeSpec,
    },
}

/// A resolved or relative time specification
#[derive(Debug, Clone)]
pub enum TimeSpec {
    DaysAgo(i64),
    WeeksAgo(i64),
    MonthsAgo(i64),
    YearsAgo(i64),
    Month(u32),
    Year(i32),
}

impl TimeSpec {
    pub fn resolve(&self) -> NaiveDate {
        let today = Local::now().date_naive();
        match self {
            TimeSpec::DaysAgo(d) => today - Duration::days(*d),
            TimeSpec::WeeksAgo(w) => today - Duration::weeks(*w),
            TimeSpec::MonthsAgo(m) => today - Months::new(*m as u32),
            TimeSpec::YearsAgo(y) => today - Months::new(*y as u32 * 12),
            TimeSpec::Month(m) => {
                NaiveDate::from_ymd_opt(today.year(), *m, 15).unwrap_or(today)
            }
            TimeSpec::Year(y) => {
                NaiveDate::from_ymd_opt(*y, today.month(), today.day())
                    .unwrap_or_else(|| NaiveDate::from_ymd_opt(*y, today.month(), 28).unwrap_or(today))
            }
        }
    }
}

/// Parse a shorthand query from positional args
pub fn parse_query(args: &[String]) -> Result<Query> {
    if args.is_empty() {
        return Err(WitError::Other(anyhow::anyhow!(
            "no query provided — try `wit <location>` or `wit --help`"
        )));
    }

    // Join args and re-split on spaces to handle "tokyo 7d" and "tokyo..boston"
    let joined = args.join(" ");
    let tokens: Vec<&str> = joined.split_whitespace().collect();

    // Check if any token contains ".." — that's either location..location or time..time
    let has_dotdot = tokens.iter().any(|t| t.contains(".."));

    if has_dotdot {
        return parse_dotdot_query(&tokens);
    }

    match tokens.len() {
        1 => {
            // Single token: either a location or could be a time? Treat as location.
            Ok(Query::Current {
                location: tokens[0].to_string(),
            })
        }
        2 => {
            // Two tokens: location + time
            let location = tokens[0].to_string();
            let time = parse_time_token(tokens[1])?;
            Ok(Query::TimeDiff {
                location,
                past: time,
            })
        }
        _ => {
            // Multi-word location name: join all but last, check if last is a time
            let last = tokens.last().unwrap();
            if let Ok(time) = parse_time_token(last) {
                let location = tokens[..tokens.len() - 1].join(" ");
                Ok(Query::TimeDiff {
                    location,
                    past: time,
                })
            } else {
                // All tokens form a location name
                let location = tokens.join(" ");
                Ok(Query::Current { location })
            }
        }
    }
}

fn parse_dotdot_query(tokens: &[&str]) -> Result<Query> {
    // Find the token with ".."
    let dotdot_idx = tokens.iter().position(|t| t.contains("..")).unwrap();
    let dotdot_token = tokens[dotdot_idx];
    let parts: Vec<&str> = dotdot_token.splitn(2, "..").collect();

    let left = parts[0];
    let right = parts[1];

    // Determine if this is location..location or time..time
    let left_is_time = parse_time_token(left).is_ok();
    let right_is_time = parse_time_token(right).is_ok();

    if left_is_time && right_is_time {
        // time..time — need location from other tokens
        let location_tokens: Vec<&str> = tokens
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != dotdot_idx)
            .map(|(_, t)| *t)
            .collect();

        if location_tokens.is_empty() {
            return Err(WitError::Other(anyhow::anyhow!(
                "time range needs a location — try `wit <location> jan..jul`"
            )));
        }

        let location = location_tokens.join(" ");
        let from = parse_time_token(left)?;
        let to = parse_time_token(right)?;
        Ok(Query::TimeRange { location, from, to })
    } else if !left_is_time && !right_is_time {
        // location..location
        let left_loc = left.to_string();
        let right_loc = right.to_string();

        // Check for optional time token
        let extra_tokens: Vec<&str> = tokens
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != dotdot_idx)
            .map(|(_, t)| *t)
            .collect();

        if extra_tokens.is_empty() {
            Ok(Query::LocationDiff {
                left: left_loc,
                right: right_loc,
            })
        } else if extra_tokens.len() == 1 {
            let time = parse_time_token(extra_tokens[0])?;
            Ok(Query::LocationDiffAt {
                left: left_loc,
                right: right_loc,
                time,
            })
        } else {
            Err(WitError::Other(anyhow::anyhow!(
                "unexpected tokens after location comparison"
            )))
        }
    } else {
        Err(WitError::Other(anyhow::anyhow!(
            "can't mix location and time in .. syntax"
        )))
    }
}

pub fn parse_time_token(s: &str) -> Result<TimeSpec> {
    let lower = s.to_lowercase();

    // Named shortcuts
    match lower.as_str() {
        "yesterday" => return Ok(TimeSpec::DaysAgo(1)),
        "last-week" | "lastweek" => return Ok(TimeSpec::DaysAgo(7)),
        "last-month" | "lastmonth" => return Ok(TimeSpec::MonthsAgo(1)),
        _ => {}
    }

    // Month names
    let months = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    for (i, name) in months.iter().enumerate() {
        if lower == *name || lower == format!("{}uary", name) || lower == format!("{}ruary", name)
            || lower == format!("{}ch", name) || lower == format!("{}il", name)
            || lower == format!("{}e", name) || lower == format!("{}y", name)
            || lower == format!("{}ust", name) || lower == format!("{}tember", name)
            || lower == format!("{}ober", name) || lower == format!("{}ember", name)
        {
            return Ok(TimeSpec::Month(i as u32 + 1));
        }
        if lower == *name {
            return Ok(TimeSpec::Month(i as u32 + 1));
        }
    }
    // Also check full month names directly
    let full_months = [
        "january", "february", "march", "april", "may", "june",
        "july", "august", "september", "october", "november", "december",
    ];
    for (i, name) in full_months.iter().enumerate() {
        if lower == *name {
            return Ok(TimeSpec::Month(i as u32 + 1));
        }
    }

    // Duration patterns: 7d, 2w, 3m, 1y, 1yr
    if let Some(num_str) = lower.strip_suffix('d') {
        if let Ok(n) = num_str.parse::<i64>() {
            return Ok(TimeSpec::DaysAgo(n));
        }
    }
    if let Some(num_str) = lower.strip_suffix('w') {
        if let Ok(n) = num_str.parse::<i64>() {
            return Ok(TimeSpec::WeeksAgo(n));
        }
    }
    if let Some(num_str) = lower.strip_suffix('m') {
        if let Ok(n) = num_str.parse::<i64>() {
            return Ok(TimeSpec::MonthsAgo(n));
        }
    }
    if let Some(num_str) = lower.strip_suffix("yr") {
        if let Ok(n) = num_str.parse::<i64>() {
            return Ok(TimeSpec::YearsAgo(n));
        }
    }
    if let Some(num_str) = lower.strip_suffix('y') {
        if let Ok(n) = num_str.parse::<i64>() {
            return Ok(TimeSpec::YearsAgo(n));
        }
    }

    // Year: 4-digit number
    if lower.len() == 4 {
        if let Ok(year) = lower.parse::<i32>() {
            if (1900..=2100).contains(&year) {
                return Ok(TimeSpec::Year(year));
            }
        }
    }

    Err(WitError::InvalidTime(s.to_string()))
}
