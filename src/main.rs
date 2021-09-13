use actix_web::{http::header, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use icalendar::{Calendar, Event};
use schedule::{get_credentials, get_lessons_by_week, list_timetables};
use std::env;

async fn get_ical(_: HttpRequest) -> impl Responder {
    let username = env::var("S_USERNAME").expect("set S_USERNAME");
    let password = env::var("S_PASSWORD").expect("set S_PASSWORD");

    let c = reqwest::Client::new();

    let creds = get_credentials(username, password).await.unwrap();

    dbg!(&creds);

    let timetables = list_timetables(&c, &creds).await.unwrap();

    let t = timetables.into_iter().next().unwrap();

    let lessons = get_lessons_by_week(&c, &creds, 2021, 38, t).await.unwrap();

    let mut cal = Calendar::new();

    for l in lessons {
        let e: Event = l.into();
        cal.push(e);
    }

    HttpResponse::Ok()
        .append_header(header::ContentType("text/calendar".parse().unwrap()))
        .body(cal.to_string())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    HttpServer::new(|| App::new().route("/basic.ics", web::get().to(get_ical)))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}
