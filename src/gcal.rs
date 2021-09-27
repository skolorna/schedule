use std::fs::OpenOptions;

use chrono::{DateTime, NaiveDate, Utc};
use reqwest::Client;
use serde::{Serialize, Deserialize};
use serde_json::Value;
use yup_oauth2::AccessToken;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    pub id: String,
    pub summary: String,
}

pub async fn list_calendars(
    c: &Client,
    access_token: &AccessToken,
) -> Result<Vec<Calendar>, reqwest::Error> {
    #[derive(Debug, Deserialize)]
    struct Res {
        items: Vec<Calendar>,
    }

    let res: Res = c
        .get("https://www.googleapis.com/calendar/v3/users/me/calendarList")
        .bearer_auth(access_token.as_str())
        .send()
        .await?
        .json()
        .await?;

    Ok(res.items)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Timestamp {
#[serde(rename_all = "camelCase")]
		WithTime {
			date_time: DateTime<Utc>,
		},
#[serde(rename_all = "camelCase")]
		DateOnly {
			date: NaiveDate,
		}
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
	pub end: Timestamp,
	pub start: Timestamp,
	pub summary: Option<String>,
	pub location: Option<String>,
	pub description: Option<String>,
	pub recurrence: Option<Vec<String>>,
}

pub async fn insert_event(
	c: &Client,
	access_token: &AccessToken,
	calendar_id: &str,
	event: Event,
) -> Result<(), reqwest::Error> {
	let url = format!("https://www.googleapis.com/calendar/v3/calendars/{}/events", calendar_id);

	let res: Value = c.post(&url).bearer_auth(access_token.as_str()).json(&event).send().await?.json().await?;

	dbg!(res);

	todo!();
}
