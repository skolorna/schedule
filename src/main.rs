use std::{convert::TryInto, env};

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Weekday};
use dotenv::dotenv;
use fantoccini::{Client, Locator};
use icalendar::{Calendar, Component, Event};
use reqwest::header::{self, HeaderMap, HeaderValue};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug)]
struct ScheduleCredentials {
    sm_session: String,
    asp_session: String,
    scope: String,
}

impl ScheduleCredentials {
    pub fn as_headers(&self) -> HeaderMap {
        let mut map = HeaderMap::new();
        map.insert(
            header::COOKIE,
            format!(
                "SMSESSION={}; ASP.NET_SessionId={}",
                self.sm_session, self.asp_session
            )
            .try_into()
            .unwrap(),
        );
        map.insert("X-Scope", HeaderValue::from_str(&self.scope).unwrap());
        map
    }
}

async fn get_credentials(
    username: &str,
    password: &str,
) -> Result<ScheduleCredentials, fantoccini::error::CmdError> {
    let mut c = Client::new("http://localhost:4444")
        .await
        .expect("failed to connect to WebDriver");

    c.goto("https://fnsservicesso1.stockholm.se/sso-ng/saml-2.0/authenticate?customer=https://login001.stockholm.se&targetsystem=TimetableViewer").await?;

    // Switch to student login
    c.wait()
        .for_element(Locator::LinkText("Elever"))
        .await?
        .click()
        .await?;

    // Select password login
    c.wait()
        .for_element(Locator::LinkText("Logga in med användarnamn och lösenord"))
        .await?
        .click()
        .await?;

    // Enter login details
    c.wait()
        .for_element(Locator::Css("input[name=user]"))
        .await?;
    c.form(Locator::Css("form"))
        .await?
        .set_by_name("user", username)
        .await?
        .set_by_name("password", password)
        .await?
        .submit()
        .await?;

    c.wait()
        .for_url(Url::parse("https://fns.stockholm.se/ng/portal/start").unwrap())
        .await?;

    let sm_session = c.get_named_cookie("SMSESSION").await?;
    let sm_session = sm_session.value();
    let asp_session = c.get_named_cookie("ASP.NET_SessionId").await?;
    let asp_session = asp_session.value();

    c.goto("https://fnsservicesso1.stockholm.se/sso-ng/saml-2.0/authenticate?customer=https://login001.stockholm.se&targetsystem=TimetableViewer").await?;

    let html = reqwest::Client::new()
        .get("https://fns.stockholm.se/ng/timetable/timetable-viewer/fns.stockholm.se/")
        .header(header::COOKIE, format!("SMSESSION={}", sm_session))
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
        .unwrap();

    c.close().await?;

    Ok(ScheduleCredentials {
        sm_session: sm_session.to_owned(),
        asp_session: asp_session.to_owned(),
        scope: scope.to_owned(),
    })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Timetable {
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

async fn list_timetables(creds: &ScheduleCredentials) -> Result<Vec<Timetable>, reqwest::Error> {
    let client = reqwest::Client::new();

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

async fn get_render_key(creds: &ScheduleCredentials) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();

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
struct Lesson {
    start: NaiveDateTime,
    end: NaiveDateTime,
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
            start: date.and_time(start),
            end: date.and_time(end),
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

async fn get_lessons(
    creds: &ScheduleCredentials,
    year: i32,
    week: u32,
    info: Timetable,
) -> Result<Vec<Lesson>, reqwest::Error> {
    let render_key = get_render_key(creds).await?;

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

    let client = reqwest::Client::new();

    let ResWrapper { data } = client
        .post("https://fns.stockholm.se/ng/api/render/timetable")
        .headers(creds.as_headers())
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

#[tokio::main]
async fn main() -> Result<(), fantoccini::error::CmdError> {
    dotenv().ok();

    let username = env::var("S_USERNAME").expect("set S_USERNAME");
    let password = env::var("S_PASSWORD").expect("set S_PASSWORD");

    let creds = get_credentials(&username, &password).await?;

    let timetables = list_timetables(&creds).await.unwrap();

    let t = timetables.into_iter().next().unwrap();

    let lessons = get_lessons(&creds, 2021, 37, t).await.unwrap();

    let mut cal = Calendar::new();

    for l in lessons {
        let e: Event = l.into();
        cal.push(e);
    }

    println!("{}", cal);

    Ok(())
}
