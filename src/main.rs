// use actix_web::{http::header, web, App, HttpResponse, HttpServer, Responder};
// use chrono::{Datelike, NaiveDate};
use dotenv::dotenv;
// use icalendar::{Calendar, Event};
use schedule::{auth::get_credentials, get_lessons};
use yup_oauth2::ConsoleApplicationSecret;
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

    let oauth2_json = include_str!("../oauth2.json");
    let secret: ConsoleApplicationSecret = serde_json::from_str(oauth2_json).unwrap();
    
    let username = env::var("S_USERNAME").expect("set S_USERNAME");
    let password = env::var("S_PASSWORD").expect("set S_PASSWORD");

        let creds = get_credentials(username, password).await.unwrap();

    dbg!(secret);

    // let lessons = get_lessons(&c, &creds, info.from.iso_week(), info.to.iso_week())
    //     .await
    //     .unwrap();

        Ok(())
}
