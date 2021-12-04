use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

// https://asvz.ch/asvz_api/sport_search?_format=json&limit=999
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SportSearch {
    pub results: Vec<Result>,
    pub count: Count,
    pub facets: Vec<Facet>,
    pub state: State,
}

impl SportSearch {
    pub fn name_id_map(self) -> HashMap<String, String> {
        self.results
            .into_iter()
            .map(|result| (result.title, result.nid.to_string()))
            .collect()
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Result {
    #[serde(rename = "event_search_url")]
    pub event_search_url: String,
    pub nid: i64,
    pub slogan: String,
    pub summary: String,
    #[serde(rename = "target_url")]
    pub target_url: String,
    #[serde(rename = "tile_image")]
    pub tile_image: Vec<String>,
    #[serde(rename = "tile_image_responsive")]
    pub tile_image_responsive: String,
    pub title: String,
    #[serde(rename = "type")]
    pub type_field: String,
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
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub selected: Vec<Value>,
    pub values: Values,
    pub url: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Values {
    pub sort: String,
}
