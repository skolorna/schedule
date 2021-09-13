use actix_web::web;
use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};
use chrono_tz::Europe::Stockholm;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use icalendar::{Component, Event};
use reqwest::{header, Client};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct ScheduleCredentials {
    pub cookies: String,
    pub scope: String,
}

pub async fn get_scope(cookie: &str) -> String {
    let html = Client::new()
        .get("https://fns.stockholm.se/ng/timetable/timetable-viewer/fns.stockholm.se/")
        .header(header::COOKIE, cookie)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let doc = Html::parse_document(&html);

    let scope = doc
        .select(&Selector::parse("nova-widget").unwrap())
        .next()
        .unwrap()
        .value()
        .attr("scope")
        .unwrap()
        .to_owned();

    scope
}

pub fn get_cookies(username: String, password: String) -> String {
    let browser = Browser::new(
        LaunchOptionsBuilder::default()
            .headless(false)
            .build()
            .unwrap(),
    )
    .unwrap();

    dbg!("waiting for tab");

    let tab = browser.wait_for_initial_tab().unwrap();

    dbg!("got tab");

    tab.navigate_to("https://fnsservicesso1.stockholm.se/sso-ng/saml-2.0/authenticate?customer=https://login001.stockholm.se&targetsystem=TimetableViewer").unwrap();

    tab.wait_for_element("a.btn:nth-child(1)")
        .unwrap()
        .click()
        .unwrap();

    tab.wait_for_element("a.beta").unwrap().click().unwrap();

    tab.wait_for_element("input[name=user]")
        .unwrap()
        .type_into(&username)
        .unwrap();
    tab.wait_for_element("input[name=password]")
        .unwrap()
        .type_into(&password)
        .unwrap();
    tab.wait_for_element("button[type=submit]")
        .unwrap()
        .click()
        .unwrap();

    // tab.wait_until_navigated().unwrap();
    tab.wait_for_element(".site-navigation-header-label > h2:nth-child(1)")
        .unwrap();

    let cookies = tab
        .get_cookies()
        .unwrap()
        .into_iter()
        .map(|c| format!("{}={}", c.name, c.value))
        .collect::<Vec<_>>()
        .join("; ");

    dbg!(&cookies);
    tab.navigate_to("https://fnsservicesso1.stockholm.se/sso-ng/saml-2.0/authenticate?customer=https://login001.stockholm.se&targetsystem=TimetableViewer").unwrap();

    cookies
}

pub async fn get_credentials(username: String, password: String) -> ScheduleCredentials {
    let cookies = web::block(|| get_cookies(username, password))
        .await
        .unwrap();
    let scope = get_scope(&cookies).await;

    ScheduleCredentials { cookies, scope }
}

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
        .header(header::COOKIE, creds.cookies.to_owned())
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
        .header(header::COOKIE, creds.cookies.to_owned())
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
    pub fn weekday(&self) -> Weekday {
        match self.day_of_week_number {
            1 => Weekday::Mon,
            2 => Weekday::Tue,
            3 => Weekday::Wed,
            4 => Weekday::Thu,
            5 => Weekday::Fri,
            6 => Weekday::Sat,
            7 => Weekday::Sun,
            _ => unreachable!(),
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
    year: i32,
    week: u32,
    info: Timetable,
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
        lesson_info: Vec<S24Lesson>,
    }

    let ResWrapper { data } = client
        .post("https://fns.stockholm.se/ng/api/render/timetable")
        .header(header::COOKIE, creds.cookies.to_owned())
        .json(&RenderTimetableReq {
            render_key,
            host: "fns.stockholm.se".to_owned(),
            unit_guid: info.unit_guid,
            width: 732,
            height: 550,
            selection_type: 5,
            selection: info.person_guid,
            week,
            year,
        })
        .send()
        .await?
        .json::<ResWrapper<Res>>()
        .await?;

    let lessons = data
        .lesson_info
        .into_iter()
        .map(|l| {
            let d = NaiveDate::from_isoywd(year, week, l.weekday());
            Lesson::try_from_lesson(l, d)
        })
        .collect::<Result<Vec<Lesson>, chrono::ParseError>>()
        .expect("failed to parse");

    Ok(lessons)
}
