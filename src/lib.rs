pub mod auth;
pub mod errors;
pub mod s24;
pub mod util;

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Europe::Stockholm;
use s24::S24Lesson;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct LessonInfo {
    course: String,
    location: Option<String>,
    teacher: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Lesson {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    #[serde(flatten)]
    info: LessonInfo,
}

impl Lesson {
    fn try_from_s24_lesson(value: S24Lesson, date: NaiveDate) -> Result<Self, chrono::ParseError> {
        const FMT: &str = "%H:%M:%S";

        let start = NaiveTime::parse_from_str(&value.time_start, FMT)?;
        let end = NaiveTime::parse_from_str(&value.time_end, FMT)?;

        let mut texts = value.texts.into_iter();
        let course = texts.next().expect("noooo");
        let teacher = texts.next();
        let location = texts.next();

        Ok(Self {
            start: Stockholm
                .from_local_datetime(&date.and_time(start))
                .unwrap()
                .with_timezone(&Utc),
            end: Stockholm
                .from_local_datetime(&date.and_time(end))
                .unwrap()
                .with_timezone(&Utc),
            info: LessonInfo {
                course,
                teacher,
                location,
            },
        })
    }
}
