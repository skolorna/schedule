use std::{convert::TryInto, sync::Arc};

use reqwest::{
    cookie::{CookieStore, Jar},
    header::{self, HeaderMap},
    Client, Url,
};
use scraper::{Html, Selector};

use crate::util::parse_html_form;

#[derive(Debug)]
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

pub async fn get_credentials(
    username: &str,
    password: &str,
) -> Result<ScheduleCredentials, reqwest::Error> {
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
        .select(&Selector::parse("a.navBtn").unwrap())
        .next()
        .unwrap()
        .value()
        .attr("href")
        .unwrap();

    let res = client.get(url(href)).send().await?;
    let html = Html::parse_document(&res.text().await?);
    let href = html
        .select(&Selector::parse("a.beta").unwrap())
        .next()
        .unwrap()
        .value()
        .attr("href")
        .unwrap();

    let res = client.get(url(href)).send().await?;
    let mut form_body = parse_html_form(&res.text().await?).unwrap();

    form_body.insert("user".to_owned(), username.to_owned());
    form_body.insert("password".to_owned(), password.to_owned());
    form_body.insert("submit".to_owned(), "".to_owned());

    let res = client
        .post("https://login001.stockholm.se/siteminderagent/forms/login.fcc")
        .form(&form_body)
        .send()
        .await?;

    let form_body = parse_html_form(&res.text().await?).unwrap();

    let res = client
        .post("https://login001.stockholm.se/affwebservices/public/saml2sso")
        .form(&form_body)
        .send()
        .await?;
    let form_body = parse_html_form(&res.text().await?).unwrap();

    let _ = client
        .post("https://fnsservicesso1.stockholm.se/sso-ng/saml-2.0/response")
        .form(&form_body)
        .send()
        .await?;

    let html = client
        .get("https://fns.stockholm.se/ng/timetable/timetable-viewer/fns.stockholm.se/")
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
        .unwrap()
        .to_owned();

    let cookies = jar
        .cookies(&Url::parse("https://fns.stockholm.se").unwrap())
        .expect("no cookies for you")
        .to_str()
        .unwrap()
        .to_owned();

    Ok(ScheduleCredentials { cookies, scope })
}
