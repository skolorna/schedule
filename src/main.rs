use std::env;

use dotenv::dotenv;
use fantoccini::{Client, Locator};
use reqwest::header;
use scraper::{Html, Selector};
use url::Url;

#[derive(Debug)]
struct ScheduleCredentials {
    session: String,
    scope: String,
}

async fn get_credentials(
    username: &str,
    password: &str,
) -> Result<ScheduleCredentials, fantoccini::error::CmdError> {
    let mut c = Client::new("http://localhost:4444")
        .await
        .expect("failed to connect to WebDriver");

    c.goto("https://skolplattformen.stockholm.se").await?;

    c.find(Locator::Css("input[type=email]"))
        .await?
        .send_keys("joe@elevmail.stockholm.se")
        .await?;
    c.find(Locator::Css("input[type=submit]"))
        .await?
        .click()
        .await?;

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

    // Don't stay signed in
    c.wait()
        .for_element(Locator::Id("idBtn_Back"))
        .await?
        .click()
        .await?;

    c.wait()
        .for_url(Url::parse("https://elevstockholm.sharepoint.com/sites/skolplattformen/").unwrap())
        .await?;

    c.goto("https://fnsservicesso1.stockholm.se/sso-ng/saml-2.0/authenticate?customer=https://login001.stockholm.se&targetsystem=TimetableViewer").await?;

    let session = c.get_named_cookie("SMSESSION").await?;
    let session = session.value();

    let client = reqwest::Client::new();

    let html = client
        .get("https://fns.stockholm.se/ng/timetable/timetable-viewer/fns.stockholm.se/")
        .header(header::COOKIE, format!("SMSESSION={}", session))
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
        session: session.to_owned(),
        scope: scope.to_owned(),
    })
}

#[tokio::main]
async fn main() -> Result<(), fantoccini::error::CmdError> {
    dotenv().ok();

    let username = env::var("S_USERNAME").expect("set S_USERNAME");
    let password = env::var("S_PASSWORD").expect("set S_PASSWORD");

    let creds = get_credentials(&username, &password).await?;

    dbg!(creds);

    Ok(())
}
