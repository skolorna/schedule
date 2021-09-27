pub mod auth;
pub mod gcal;
pub mod s24;
pub mod util;
use std::collections::HashMap;

use chrono::{Date, DateTime, Datelike, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Europe::Stockholm;
use icalendar::{Component, Event};
use s24::S24Lesson;

#[derive(Debug, PartialEq)]
pub struct LessonInfo {
    course: String,
    location: Option<String>,
    teacher: Option<String>,
}

#[derive(Debug)]
pub struct LessonInstance {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    info: LessonInfo,
}

impl LessonInstance {
    fn try_from_lesson(value: S24Lesson, date: NaiveDate) -> Result<Self, chrono::ParseError> {
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

    fn is_same(&self, other: &Self) -> bool {
        self.start.time() == other.start.time()
            && self.end.time() == other.end.time()
            && self.start.weekday() == other.start.weekday()
            && self.end.weekday() == other.end.weekday()
            && self.info == other.info
    }
}

impl From<LessonInstance> for Event {
    fn from(l: LessonInstance) -> Self {
        let mut e = Event::new();
        e.summary(&l.info.course).starts(l.start).ends(l.end);

        if let Some(location) = l.info.location {
            e.location(&location);
        }

        if let Some(teacher) = l.info.teacher {
            e.description(&teacher);
        }

        e.done()
    }
}

#[derive(Debug)]
pub struct ReccuringLesson {
    pub dates: Vec<Date<Utc>>,
    pub start: NaiveTime,
    pub end: NaiveTime,
}

impl ReccuringLesson {
    pub fn from_instances(mut instances: Vec<LessonInstance>) -> Vec<ReccuringLesson> {
        instances.sort_unstable_by_key(|i| i.start);

        instances.reverse();

        let mut res: Vec<Vec<LessonInstance>> = vec![];

        while let Some(i) = instances.pop() {
            
        }

        dbg!(instances);

        todo!();
    }
}
