use std::str::FromStr;
use crate::cmd::LessonID;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use tracing::warn;

lazy_static! {
    static ref LESSON_URL_RE: Regex = Regex::new("/tn/lessons/([0-9]+)").unwrap();
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventList {
    pub results: Vec<Result>,
    pub count: Count,
    pub facets: Vec<Facet>,
    pub state: State,
}

impl EventList {
    pub fn lesson_id(&self) -> Option<LessonID> {
        match &*self.results {
            [] => None,
            [result] => result.lesson_id(),
            [result, ..] => {
                warn!("Multiply results in event_list: {:?}", &self.results);
                result.lesson_id()
            }
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Result {
    #[serde(rename = "beginner_friendly")]
    pub beginner_friendly: bool,
    pub cancelled: bool,
    #[serde(rename = "event_type")]
    pub event_type: Vec<i64>,
    #[serde(rename = "event_type_name")]
    pub event_type_name: String,
    pub facility: Vec<i64>,
    #[serde(rename = "facility_name")]
    pub facility_name: Vec<String>,
    #[serde(rename = "facility_type")]
    pub facility_type: Vec<i64>,
    #[serde(rename = "facility_type_name")]
    pub facility_type_name: Vec<String>,
    #[serde(rename = "from_date")]
    pub from_date: String,
    #[serde(rename = "general_type")]
    pub general_type: Vec<i64>,
    #[serde(rename = "general_type_name")]
    pub general_type_name: String,
    pub livestream: bool,
    pub location: String,
    pub nid: i64,
    #[serde(rename = "niveau_name")]
    pub niveau_name: String,
    #[serde(rename = "niveau_short_name")]
    pub niveau_short_name: String,
    #[serde(rename = "oe_enabled")]
    pub oe_enabled: bool,
    #[serde(rename = "oe_from_date")]
    pub oe_from_date: String,
    #[serde(rename = "oe_to_date")]
    pub oe_to_date: String,
    #[serde(rename = "places_max")]
    pub places_max: i64,
    pub searchable: bool,
    pub sport: Vec<i64>,
    #[serde(rename = "sport_name")]
    pub sport_name: String,
    pub title: String,
    #[serde(rename = "to_date")]
    pub to_date: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub url: String,
    #[serde(rename = "from_date_stamp")]
    pub from_date_stamp: i64,
    #[serde(rename = "from_date_key")]
    pub from_date_key: String,
    #[serde(rename = "to_date_stamp")]
    pub to_date_stamp: i64,
    #[serde(rename = "to_date_key")]
    pub to_date_key: String,
    #[serde(rename = "oe_from_date_stamp")]
    pub oe_from_date_stamp: i64,
    #[serde(rename = "oe_from_date_key")]
    pub oe_from_date_key: String,
    #[serde(rename = "oe_to_date_stamp")]
    pub oe_to_date_stamp: i64,
    #[serde(rename = "oe_to_date_key")]
    pub oe_to_date_key: String,
    #[serde(rename = "facility_url")]
    pub facility_url: Vec<String>,
}

impl Result {
    pub fn lesson_id(&self) -> Option<LessonID> {
        Some(LessonID::from_str(&LESSON_URL_RE.captures(&self.url)?[1]).expect("Captures non numbers"))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Count {
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Facet {
    pub id: String,
    pub label: String,
    pub terms: Vec<Term>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Term {
    pub label: String,
    pub tid: String,
    pub count: i64,
    pub query_id: String,
    pub facet_id: String,
    pub sid: String,
    pub active: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub selected: Vec<Selected>,
    pub values: Values,
    pub now: i64,
    #[serde(rename = "utc_offset")]
    pub utc_offset: i64,
    pub url: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Selected {
    pub label: Option<String>,
    pub tid: Option<String>,
    pub count: Option<i64>,
    pub query_id: Option<String>,
    pub facet_id: Option<String>,
    pub sid: String,
    pub active: Option<bool>,
    #[serde(rename = "type")]
    pub type_field: String,
    pub key: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Values {
    pub date: i64,
    pub sort: String,
    pub availability: i64,
    pub livestream: i64,
    #[serde(rename = "without_fitness")]
    pub without_fitness: i64,
}
