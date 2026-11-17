use crate::festival;
use chrono::{Datelike, NaiveDate, Weekday};

type Schedule = (&'static [(u32, u32)], &'static [u32]);

/// Each holiday range uses inclusive MMDD values within the given year.
fn published_schedule(year: i32) -> Option<Schedule> {
    match year {
        // State Council notices, 国办发明电〔2025〕7号 and 〔2026〕10号.
        2026 => Some((
            &[
                (101, 103),
                (215, 223),
                (404, 406),
                (501, 505),
                (619, 621),
                (925, 927),
                (1001, 1007),
            ],
            &[104, 214, 228, 509, 920, 1010],
        )),
        2027 => Some((
            &[
                (101, 103),
                (205, 213),
                (403, 405),
                (501, 505),
                (609, 609),
                (915, 915),
                (1001, 1007),
            ],
            &[131, 214, 508, 926, 1009],
        )),
        _ => None,
    }
}

pub(crate) fn is_workday(date: NaiveDate) -> bool {
    let weekday = !matches!(date.weekday(), Weekday::Sat | Weekday::Sun);
    if let Some((holidays, makeup)) = published_schedule(date.year()) {
        let day = date.month() * 100 + date.day();
        return makeup.contains(&day)
            || (weekday
                && !holidays
                    .iter()
                    .any(|&(from, through)| (from..=through).contains(&day)));
    }
    weekday && !festival::is_modeled_holiday(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_published_holidays_and_weekend_makeup_days() {
        let date = |year, month, day| NaiveDate::from_ymd_opt(year, month, day).unwrap();
        assert!(is_workday(date(2026, 2, 14)));
        assert!(!is_workday(date(2026, 2, 23)));
        assert!(!is_workday(date(2027, 2, 5)));
        assert!(is_workday(date(2027, 2, 14)));
        assert!(!is_workday(date(2027, 6, 9)));
        assert!(is_workday(date(2027, 6, 10)));
    }

    #[test]
    fn models_weekends_and_lunar_holidays_for_other_years() {
        let date = |month, day| NaiveDate::from_ymd_opt(2028, month, day).unwrap();
        assert!(is_workday(date(1, 25)));
        assert!(!is_workday(date(1, 26)));
        assert!(!is_workday(date(2, 1)));
        assert!(is_workday(date(2, 2)));
        assert!(!is_workday(date(2, 5)));
    }
}
