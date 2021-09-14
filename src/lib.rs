pub mod auth;
pub mod util;

use std::convert::TryInto;

use auth::ScheduleCredentials;
use chrono::{DateTime, Datelike, Duration, IsoWeek, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};
use chrono_tz::Europe::Stockholm;
use icalendar::{Component, Event};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timetable {
    first_name: String,
    last_name: String,
    person_guid: String,
    school_guid: String,
    #[serde(rename = "schoolID")]
    school_id: String,
    #[serde(rename = "timetableID")]
    timetable_id: String,
    unit_guid: String,
}

#[derive(Debug, Deserialize)]
struct ResWrapper<T> {
    data: T,
}

pub async fn list_timetables(
    client: &reqwest::Client,
    creds: &ScheduleCredentials,
) -> Result<Vec<Timetable>, reqwest::Error> {
    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Req {
        get_personal_timetables_request: InnerReq,
    }

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct InnerReq {
        host_name: String,
    }

    let res = client
        .post("https://fns.stockholm.se/ng/api/services/skola24/get/personal/timetables")
        .json(&Req {
            get_personal_timetables_request: InnerReq {
                host_name: "fns.stockholm.se".to_owned(),
            },
        })
        .headers(creds.as_headers())
        .send()
        .await?;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Res {
        get_personal_timetables_response: InnerRes,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct InnerRes {
        student_timetables: Vec<Timetable>,
    }

    let ResWrapper { data } = res.json::<ResWrapper<Res>>().await?;

    Ok(data.get_personal_timetables_response.student_timetables)
}

async fn get_render_key(
    client: &reqwest::Client,
    creds: &ScheduleCredentials,
) -> Result<String, reqwest::Error> {
    #[derive(Debug, Deserialize)]
    struct Res {
        key: String,
    }

    let ResWrapper { data } = client
        .post("https://fns.stockholm.se/ng/api/get/timetable/render/key")
        .headers(creds.as_headers())
        .json("")
        .send()
        .await?
        .json::<ResWrapper<Res>>()
        .await?;

    Ok(data.key)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct S24Lesson {
    guid_id: String,
    texts: Vec<String>,
    time_start: String,
    time_end: String,
    day_of_week_number: u8,
    block_name: String,
}

impl S24Lesson {
    pub fn weekday(&self) -> Option<Weekday> {
        match self.day_of_week_number {
            1 => Some(Weekday::Mon),
            2 => Some(Weekday::Tue),
            3 => Some(Weekday::Wed),
            4 => Some(Weekday::Thu),
            5 => Some(Weekday::Fri),
            6 => Some(Weekday::Sat),
            7 => Some(Weekday::Sun),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Lesson {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    course: String,
    location: Option<String>,
    teacher: Option<String>,
}

impl Lesson {
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
            course,
            teacher,
            location,
        })
    }
}

impl From<Lesson> for Event {
    fn from(l: Lesson) -> Self {
        let mut e = Event::new();
        e.summary(&l.course).starts(l.start).ends(l.end);

        if let Some(location) = l.location {
            e.location(&location);
        }

        if let Some(teacher) = l.teacher {
            e.description(&teacher);
        }

        e.done()
    }
}

pub async fn get_lessons_by_week(
    client: &reqwest::Client,
    creds: &ScheduleCredentials,
    iso_week: IsoWeek,
    info: &Timetable,
) -> Result<Vec<Lesson>, reqwest::Error> {
    let render_key = get_render_key(client, creds).await?;

    #[derive(Debug, Serialize)]
    #[serde(rename_all = "camelCase")]
    struct RenderTimetableReq {
        render_key: String,
        host: String,
        unit_guid: String,
        width: u32,
        height: u32,
        selection_type: u8,
        selection: String,
        week: u32,
        year: i32,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Res {
        lesson_info: Option<Vec<S24Lesson>>,
    }

    let ResWrapper { data } = client
        .post("https://fns.stockholm.se/ng/api/render/timetable")
        .headers(creds.as_headers())
        .json(&RenderTimetableReq {
            render_key,
            host: "fns.stockholm.se".to_owned(),
            unit_guid: info.unit_guid.to_owned(),
            width: 732,
            height: 550,
            selection_type: 5,
            selection: info.person_guid.to_owned(),
            week: iso_week.week(),
            year: iso_week.year(),
        })
        .send()
        .await?
        .json::<ResWrapper<Res>>()
        .await?;

    let lessons = data
        .lesson_info
        .unwrap_or(vec![])
        .into_iter()
        .filter_map(|l| {
            let d = NaiveDate::from_isoywd(iso_week.year(), iso_week.week(), l.weekday()?);
            Some(Lesson::try_from_lesson(l, d))
        })
        .collect::<Result<Vec<Lesson>, chrono::ParseError>>()
        .expect("failed to parse");

    Ok(lessons)
}

pub async fn get_lessons(
    client: &reqwest::Client,
    creds: &ScheduleCredentials,
    from: IsoWeek,
    to: IsoWeek,
) -> Result<Vec<Lesson>, reqwest::Error> {
    let timetables = list_timetables(&client, &creds).await.unwrap();
    let t = timetables.into_iter().next().unwrap();
    let from = NaiveDate::from_isoywd(from.year(), from.week(), Weekday::Mon);
    let to = NaiveDate::from_isoywd(to.year(), to.week(), Weekday::Sun);

    let num_weeks: u32 = (to - from)
        .num_weeks()
        .try_into()
        .expect("from cannot be after to");

    let mut lessons: Vec<Lesson> = vec![];

    for w in 0..=num_weeks {
        let d = from + Duration::weeks(w.into());
        let mut l = get_lessons_by_week(client, creds, d.iso_week(), &t).await?;

        lessons.append(&mut l);
    }

    Ok(lessons)
}
