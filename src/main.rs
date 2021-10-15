use std::{env, net::SocketAddr};

use actix_cors::Cors;
use actix_web::{
    http::header::{CacheControl, CacheDirective, ContentType},
    web, App, HttpResponse, HttpServer,
};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use chrono::{Datelike, Duration, NaiveDate, Utc, Weekday};
use dotenv::dotenv;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use reqwest::Client;
use schedule::{
    auth::{get_credentials, ScheduleCredentials},
    errors::{AppError, AppResult},
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
) -> AppResult<HttpResponse> {
    let encoding_key = EncodingKey::from_secret(env::var("JWT_SECRET").unwrap().as_bytes());

    let creds = get_credentials(&username, &password).await?;
    let token = jsonwebtoken::encode(
        &Header::new(Algorithm::HS256),
        &AuthTokenClaims {
            data: creds.encrypt(),
            exp: (Utc::now() + Duration::minutes(15)).timestamp(),
        },
        &encoding_key,
    )
    .map_err(|_| AppError::InternalError)?;

    Ok(HttpResponse::Ok()
        .insert_header(CacheControl(vec![CacheDirective::Private]))
        .content_type(ContentType::plaintext())
        .body(token))
}

#[derive(Debug, Deserialize)]
struct LessonsQuery {
    year: i32,
    week: u32,
}

async fn get_lessons_endpoint(
    auth: BearerAuth,
    info: web::Query<LessonsQuery>,
) -> AppResult<HttpResponse> {
    let week = NaiveDate::from_isoywd(info.year, info.week, Weekday::Mon).iso_week();

    let jwt_secret = env::var("JWT_SECRET").unwrap();
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_bytes());

    let claims: AuthTokenClaims = jsonwebtoken::decode(
        auth.token(),
        &decoding_key,
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| AppError::InvalidToken)?
    .claims;

    let creds = ScheduleCredentials::decrypt(&claims.data).map_err(|_| AppError::InvalidToken)?;

    let lessons = get_lessons(&Client::new(), &creds, week).await?;

    Ok(HttpResponse::Ok().json(lessons))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let socket = SocketAddr::new("0.0.0.0".parse().unwrap(), 8000);

    eprintln!("Binding {}", socket);

    HttpServer::new(|| {
        App::new()
            .wrap(Cors::permissive())
            .route("/auth", web::post().to(authenticate))
            .route("/lessons", web::get().to(get_lessons_endpoint))
    })
    .bind(socket)?
    .run()
    .await
}
