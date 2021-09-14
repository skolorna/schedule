use std::collections::HashMap;

use lazy_static::lazy_static;
use scraper::{ElementRef, Html, Selector};

lazy_static! {
    static ref S_FORM: Selector = Selector::parse("form").unwrap();
    static ref S_INPUT: Selector = Selector::parse("input").unwrap();
}

pub fn extract_form_fields(form: &ElementRef) -> HashMap<String, String> {
    form.select(&S_INPUT)
        .filter_map(|e| {
            let v = e.value();
            Some((v.attr("name")?.to_owned(), v.attr("value")?.to_owned()))
        })
        .collect()
}

pub fn parse_html_form(html: &str) -> Option<HashMap<String, String>> {
    let html = Html::parse_document(html);
    let form = html.select(&S_FORM).next()?;
    Some(extract_form_fields(&form))
}
