use chrono::{Datelike, Duration, IsoWeek, NaiveDate, Utc};
// use actix_web::{http::header, web, App, HttpResponse, HttpServer, Responder};
// use chrono::{Datelike, NaiveDate};
use dotenv::dotenv;
use reqwest::Client;
// use icalendar::{Calendar, Event};
use schedule::{ReccuringLesson, auth::get_credentials, gcal::{insert_event, list_calendars, Event, Timestamp}, s24::get_lessons};
use serde_json::json;
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};
// use serde::Deserialize;
use std::env;

// #[derive(Debug, Deserialize)]
// struct GetCalendarInfo {
//     from: NaiveDate,
//     to: NaiveDate,
// }

// async fn get_ical(web::Query(info): web::Query<GetCalendarInfo>) -> impl Responder {
//     let username = env::var("S_USERNAME").expect("set S_USERNAME");
//     let password = env::var("S_PASSWORD").expect("set S_PASSWORD");

//     let c = reqwest::Client::new();

//     let creds = get_credentials(username, password).await.unwrap();

//     // let lessons = get_lessons_by_week(&c, &creds, 2021, 38, t).await.unwrap();
//     let lessons = get_lessons(&c, &creds, info.from.iso_week(), info.to.iso_week())
//         .await
//         .unwrap();

//     let mut cal = Calendar::new();

//     for l in lessons {
//         let e: Event = l.into();
//         cal.push(e);
//     }

//     HttpResponse::Ok()
//         .append_header(header::ContentType("text/calendar".parse().unwrap()))
//         .body(cal.to_string())
// }

// #[actix_web::main]
// async fn main() -> std::io::Result<()> {
//     dotenv().ok();

//     HttpServer::new(|| App::new().route("/basic.ics", web::get().to(get_ical)))
//         .bind(("0.0.0.0", 8080))?
//         .run()
//         .await
// }

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    // let secret = yup_oauth2::read_application_secret("oauth2.json").await.unwrap();

    // let mut auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
    // .persist_tokens_to_disk("tokencache.json")
    // .build()
    // .await
    // .unwrap();

    // let scopes = &["https://www.googleapis.com/auth/calendar"];

    // // token(<scopes>) is the one important function of this crate; it does everything to
    // // obtain a token that can be sent e.g. as Bearer token.
    // let token = auth.token(scopes).await.unwrap();

    let client = Client::new();

    // let calendars = list_calendars(&client, &token).await.unwrap();

    // dbg!(&calendars);

    // let cal = calendars.into_iter().find(|c| c.summary == "Experimental").unwrap();

    // let res = insert_event(&client, &token, &cal.id, Event {
    //     start: Timestamp::WithTime {
    //         date_time: Utc::now(),
    //     },
    //     end: Timestamp::WithTime {
    //         date_time: Utc::now() + Duration::hours(2),
    //     },
    //     summary: Some("Sleep".into()),
    //     location: None,
    //     description: None,
    //     recurrence: None,
    // }).await;

    // dbg!(res);

    let username = env::var("S_USERNAME").expect("set S_USERNAME");
    let password = env::var("S_PASSWORD").expect("set S_PASSWORD");

    let creds = get_credentials(username, password).await.unwrap();

    let lessons = get_lessons(
        &client,
        &creds,
        NaiveDate::from_ymd(2021, 9, 1).iso_week(),
        NaiveDate::from_ymd(2021, 12, 31).iso_week(),
    )
    .await
    .unwrap();

    dbg!(ReccuringLesson::from_instances(lessons));

    Ok(())
}
