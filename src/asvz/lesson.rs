use crate::asvz::api::lesson::{LessonData, LessonError};
use crate::asvz::api::search::EventList;
use crate::asvz::api::sport::SportSearch;
use crate::asvz::error::AsvzError;
use crate::cmd::LessonID;
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Client;
use std::collections::HashMap;
use tracing::{instrument, trace, warn};
use url::Url;
use reqwest_middleware::ClientWithMiddleware;

lazy_static! {
    static ref LOCATION_URL_RE: Regex = Regex::new("/anlage/([0-9]+)-").unwrap();
    static ref SPORT_URL_RE: Regex = Regex::new("/sport/([0-9]+)-").unwrap();
    static ref SEARCH_URL_TEMPLATE: Url =
        Url::parse("https://www.asvz.ch/asvz_api/event_search?_format=json&limit=1").unwrap();
    static ref SPORT_SEARCH_URL: Url =
        Url::parse("https://asvz.ch/asvz_api/sport_search?_format=json&limit=999").unwrap();
}

#[instrument(skip(client))]
pub async fn lesson_data(client: &ClientWithMiddleware, id: &LessonID) -> Result<LessonData, AsvzError> {
    trace!("fetching lesson data");
    let url = format!(
        "https://schalter.asvz.ch/tn-api/api/Lessons/{}",
        id.as_str()
    );
    let response = client.get(url).send().await?;
    let full = response.bytes().await?;
    if let Ok(data) = serde_json::from_slice::<LessonData>(&full) {
        Ok(data)
    } else if let Ok(err) = serde_json::from_slice::<LessonError>(&full) {
        Err(AsvzError::Lesson(err))
    } else {
        warn!(
            "Unable to decode: {}",
            String::from_utf8_lossy(full.as_ref())
        );
        Err(AsvzError::UnexpectedFormat)
    }
}
// https://www.asvz.ch/asvz_api/event_search?_format=json&limit=60&f[0]=sport:122920&f[1]=facility:45613&date=2021-11-08 06:35
#[instrument(skip(client))]
pub async fn search_data(
    client: &ClientWithMiddleware,
    id: &LessonID,
    offset: i64,
) -> Result<EventList, AsvzError> {
    trace!("fetching search data");
    let sport_data = get_sport_data(client).await?;
    let lesson_data = lesson_data(client, id).await?;
    // dbg!(&lesson_data);
    let next_date = LessonData::str_to_datetime(&lesson_data.data.starts)
        .map_err(|_| AsvzError::UnexpectedFormat)?
        + chrono::Duration::weeks(offset);

    let facility_url = match &*lesson_data.data.facilities {
        [] => return Err(AsvzError::UnexpectedFormat),
        [facility] => &facility.url,
        [facility, ..] => {
            warn!(
                "There are multiple facilities: {:?}",
                &lesson_data.data.facilities
            );
            &facility.url
        }
    };

    let facility_id = &LOCATION_URL_RE
        .captures(facility_url)
        .ok_or(AsvzError::UnexpectedFormat)?[1];

    let sport_id = sport_data
        .get(&lesson_data.data.sport_name)
        .ok_or(AsvzError::UnexpectedFormat)?;

    let mut search_url = SEARCH_URL_TEMPLATE.clone();
    search_url
        .query_pairs_mut()
        .append_pair("f[0]", &format!("sport:{}", sport_id))
        .append_pair("f[1]", &format!("facility:{}", facility_id))
        .append_pair("date", &next_date.format("%Y-%m-%d %H:%M").to_string());

    let event_list = client.get(search_url).send().await?.json().await?;

    Ok(event_list)
}

pub async fn get_sport_data(client: &ClientWithMiddleware) -> Result<HashMap<String, String>, AsvzError> {
    trace!("get_sport_data");
    let sport_search: SportSearch = client
        .get(SPORT_SEARCH_URL.clone())
        .send()
        .await?
        .json()
        .await?;
    Ok(sport_search.name_id_map())
}
