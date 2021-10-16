use crate::{
    auth::{ScheduleCredentials, SessionIsh},
    Lesson,
};
use chrono::{IsoWeek, NaiveDate, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timetable {
    // first_name: String,
    // last_name: String,
    person_guid: String,
    // school_guid: String,
    // #[serde(rename = "schoolID")]
    // school_id: String,
    // #[serde(rename = "timetableID")]
    // timetable_id: String,
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
pub(super) struct S24Lesson {
    // pub guid_id: String,
    pub texts: Vec<String>,
    pub time_start: String,
    pub time_end: String,
    pub day_of_week_number: u8,
    // pub block_name: String,
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

pub async fn get_lessons_by_week(
    client: &reqwest::Client,
    SessionIsh {
        credentials,
        timetable,
    }: &SessionIsh,
    iso_week: IsoWeek,
) -> Result<Vec<Lesson>, reqwest::Error> {
    let render_key = get_render_key(client, credentials).await?;

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
        .headers(credentials.as_headers())
        .json(&RenderTimetableReq {
            render_key,
            host: "fns.stockholm.se".to_owned(),
            unit_guid: timetable.unit_guid.to_owned(),
            width: 732,
            height: 550,
            selection_type: 5,
            selection: timetable.person_guid.to_owned(),
            week: iso_week.week(),
            year: iso_week.year(),
        })
        .send()
        .await?
        .json::<ResWrapper<Res>>()
        .await?;

    let lessons = data
        .lesson_info
        .unwrap_or_default()
        .into_iter()
        .filter_map(|l| {
            let d = NaiveDate::from_isoywd(iso_week.year(), iso_week.week(), l.weekday()?);
            Some(Lesson::try_from_s24_lesson(l, d))
        })
        .collect::<Result<Vec<Lesson>, chrono::ParseError>>()
        .expect("failed to parse");

    Ok(lessons)
}
