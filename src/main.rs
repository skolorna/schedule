use std::env;

use actix_cors::Cors;
use actix_web::{web, App, HttpResponse, HttpServer};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use chrono::{Datelike, Duration, NaiveDate, Utc, Weekday};
use dotenv::dotenv;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use reqwest::Client;
use schedule::{
    auth::{get_credentials, ScheduleCredentials},
    s24::get_lessons,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct AuthRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AuthTokenClaims {
    data: String,
    exp: i64,
}

async fn authenticate(
    web::Json(AuthRequest { username, password }): web::Json<AuthRequest>,
) -> HttpResponse {
    let encoding_key = EncodingKey::from_secret(env::var("JWT_SECRET").unwrap().as_bytes());

    let creds = get_credentials(&username, &password).await.unwrap();
    let token = jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &AuthTokenClaims {
            data: creds.encrypt(),
            exp: (Utc::now() + Duration::minutes(15)).timestamp(),
        },
        &encoding_key,
    )
    .unwrap();

    HttpResponse::Ok().body(token)
}

#[derive(Debug, Deserialize)]
struct LessonsQuery {
    year: i32,
    week: u32,
}

async fn get_lessons_endpoint(auth: BearerAuth, info: web::Query<LessonsQuery>) -> HttpResponse {
    let week = NaiveDate::from_isoywd(info.year, info.week, Weekday::Mon).iso_week();

    let jwt_secret = env::var("JWT_SECRET").unwrap();
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());

    let claims: AuthTokenClaims = jsonwebtoken::decode(
        auth.token(),
        &decoding_key,
        &Validation::new(Algorithm::HS256),
    )
    .unwrap()
    .claims;

    let creds = ScheduleCredentials::decrypt(&claims.data).unwrap();

    let lessons = get_lessons(&Client::new(), &creds, week).await.unwrap();

    HttpResponse::Ok().json(lessons)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    HttpServer::new(|| {
        App::new()
            .wrap(Cors::permissive())
            .route("/auth", web::post().to(authenticate))
            .route("/lessons", web::get().to(get_lessons_endpoint))
    })
    .bind(("0.0.0.0", 8000))?
    .run()
    .await
}
