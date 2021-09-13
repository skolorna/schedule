use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};
use chrono_tz::Europe::Stockholm;
use fantoccini::{Client, Locator};
use icalendar::{Component, Event};
use reqwest::header::{self, HeaderMap, HeaderValue};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use url::Url;

#[derive(Debug)]
pub struct ScheduleCredentials {
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

pub async fn get_credentials(
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
