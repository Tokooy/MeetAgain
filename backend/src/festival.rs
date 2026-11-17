use chinese_lunisolar_calendar::LunisolarDate;
use chrono::{Datelike, Days, NaiveDate};

const LEAD_DAYS: i64 = 21;
const HOLIDAY_DAYS: i64 = 7;
const COOLDOWN_DAYS: i64 = 7;
const PEAK_BOOST: f64 = 0.75;

pub(crate) fn parse_start(value: &str, ticks: u32) -> Result<NaiveDate, &'static str> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| "must be a Gregorian date in YYYY-MM-DD format")?;
    if date.to_string() != value {
        return Err("must be a Gregorian date in YYYY-MM-DD format");
    }
    let end = date.checked_add_days(Days::new(u64::from(ticks)));
    if date.year() < 1901 || end.is_none_or(|end| end.year() > 2099) {
        return Err("the entire simulation must fall within 1901 through 2099");
    }
    Ok(date)
}

pub(crate) fn date_seed(date: NaiveDate) -> u64 {
    u64::from(date.year() as u32) * 10_000 + u64::from(date.month() * 100 + date.day())
}

fn new_year(year: i32) -> NaiveDate {
    LunisolarDate::from_ymd(year as u16, 1, false, 1)
        .expect("validated calendar year")
        .to_naive_date()
}

pub(crate) fn is_modeled_holiday(date: NaiveDate) -> bool {
    let days = date.signed_duration_since(new_year(date.year())).num_days();
    (0..HOLIDAY_DAYS).contains(&days)
}

fn festival_weight(days_from_new_year: i64) -> f64 {
    if days_from_new_year < 0 {
        (1.0 + days_from_new_year as f64 / LEAD_DAYS as f64).max(0.0)
    } else {
        let days_since_holiday = (days_from_new_year - (HOLIDAY_DAYS - 1)).max(0);
        (1.0 - days_since_holiday as f64 / COOLDOWN_DAYS as f64).max(0.0)
    }
}

pub(crate) fn return_probability(date: NaiveDate, baseline: f64) -> f64 {
    let since = |year| date.signed_duration_since(new_year(year)).num_days();
    // In a year with an early Lunar New Year, the lead-in can begin in December.
    let weight = festival_weight(since(date.year())).max(festival_weight(since(date.year() + 1)));
    baseline + (1.0 - baseline) * PEAK_BOOST * weight
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lunar_new_year_matches_observatory_calendar() {
        for (year, month, day) in [
            (2026, 2, 17),
            (2027, 2, 6),
            (2028, 1, 26),
            (2033, 1, 31),
            (2042, 1, 22),
        ] {
            assert_eq!(
                new_year(year),
                NaiveDate::from_ymd_opt(year, month, day).unwrap()
            );
        }
    }

    #[test]
    fn boost_rises_holds_for_seven_days_and_falls() {
        let baseline = 0.6;
        let peak = 0.9;
        let probability =
            |offset| return_probability(new_year(2027) + chrono::TimeDelta::days(offset), baseline);
        assert_eq!(probability(-22), baseline);
        assert_eq!(probability(-21), baseline);
        assert!(probability(-14) < probability(-7));
        assert!(probability(-7) < probability(0));
        for offset in 0..7 {
            assert!((probability(offset) - peak).abs() < 1e-12);
        }
        assert!(probability(7) < probability(6));
        assert!(probability(12) > baseline);
        assert_eq!(probability(13), baseline);
        assert_eq!(return_probability(new_year(2027), 1.0), 1.0);
    }

    #[test]
    fn validates_complete_simulation_range_and_gregorian_dates() {
        assert!(parse_start("2028-02-29", 10_958).is_ok());
        assert!(parse_start("2027-02-29", 1).is_err());
        assert!(parse_start("2027-2-03", 1).is_err());
        assert!(parse_start("1900-12-31", 1).is_err());
        assert!(parse_start("2099-12-30", 1).is_ok());
        assert!(parse_start("2099-12-30", 2).is_err());
    }
}
