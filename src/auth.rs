use lazy_static::lazy_static;
use reqwest::{
    cookie::{CookieStore, Jar},
    header::{self, HeaderMap},
    Client, Url,
};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, sync::Arc};

use crate::{
    errors::{AppError, AppResult},
    s24::Timetable,
    util::parse_html_form,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleCredentials {
    pub cookies: String,
    pub scope: String,
}

impl ScheduleCredentials {
    pub fn as_headers(&self) -> HeaderMap {
        let mut map = HeaderMap::new();
        map.insert(header::COOKIE, self.cookies.to_owned().try_into().unwrap());
        map.insert(
            "X-Scope",
            header::HeaderValue::from_str(&self.scope).unwrap(),
        );
        map
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionIsh {
    pub credentials: ScheduleCredentials,
    pub timetable: Timetable,
}

impl SessionIsh {
    pub fn encrypt(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn decrypt(ciphertext: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(ciphertext)
    }
}

pub async fn get_credentials(username: &str, password: &str) -> AppResult<ScheduleCredentials> {
    lazy_static! {
        static ref A_NAVBTN: Selector = Selector::parse("a.navBtn").unwrap();
        static ref A_BETA: Selector = Selector::parse("a.beta").unwrap();
        static ref NOVA_WIDGET: Selector = Selector::parse("nova-widget").unwrap();
        static ref COOKIE_URL: Url = Url::parse("https://fns.stockholm.se").unwrap();
    }

    fn url(href: &str) -> String {
        format!(
            "https://login001.stockholm.se/siteminderagent/forms/{}",
            href
        )
    }

    let jar = Arc::new(Jar::default());

    let client = Client::builder().cookie_provider(jar.clone()).build()?;

    let res = client.get("https://fnsservicesso1.stockholm.se/sso-ng/saml-2.0/authenticate?customer=https://login001.stockholm.se&targetsystem=TimetableViewer").send().await?;
    let html = Html::parse_document(&res.text().await?);
    let href = html
        .select(&A_NAVBTN)
        .next()
        .map(|e| e.value().attr("href"))
        .flatten()
        .ok_or(AppError::InternalError)?;

    let res = client.get(url(href)).send().await?;
    let html = Html::parse_document(&res.text().await?);
    let href = html
        .select(&A_BETA)
        .next()
        .map(|e| e.value().attr("href"))
        .flatten()
        .ok_or(AppError::InternalError)?;

    let res = client.get(url(href)).send().await?;
    let mut form_body = parse_html_form(&res.text().await?).ok_or(AppError::InternalError)?;

    form_body.insert("user".to_owned(), username.to_owned());
    form_body.insert("password".to_owned(), password.to_owned());
    form_body.insert("submit".to_owned(), "".to_owned());

    let res = client
        .post("https://login001.stockholm.se/siteminderagent/forms/login.fcc")
        .form(&form_body)
        .send()
        .await?;

    let form_body = parse_html_form(&res.text().await?).ok_or(AppError::InvalidUsernamePassword)?;

    let res = client
        .post("https://login001.stockholm.se/affwebservices/public/saml2sso")
        .form(&form_body)
        .send()
        .await?;
    let form_body = parse_html_form(&res.text().await?).ok_or(AppError::InvalidUsernamePassword)?;

    let _ = client
        .post("https://fnsservicesso1.stockholm.se/sso-ng/saml-2.0/response")
        .form(&form_body)
        .send()
        .await?;

    let html = client
        .get("https://fns.stockholm.se/ng/timetable/timetable-viewer/fns.stockholm.se/")
        .send()
        .await?
        .text()
        .await?;
    let doc = Html::parse_document(&html);

    let scope = doc
        .select(&NOVA_WIDGET)
        .next()
        .map(|e| e.value().attr("scope"))
        .flatten()
        .ok_or(AppError::InternalError)?
        .to_owned();

    let cookies = jar
        .cookies(&COOKIE_URL)
        .ok_or(AppError::InternalError)?
        .to_str()
        .map_err(|_| AppError::InternalError)?
        .to_owned();

    Ok(ScheduleCredentials { cookies, scope })
}
