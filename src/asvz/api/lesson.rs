use chrono::DateTime;
use chrono::ParseError;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonData {
    pub data: Data,
}

impl LessonData {
    fn str_to_timestamp(date: &str) -> Result<i64, ParseError> {
        DateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S%z").map(|d| {
            let timestamp = d.timestamp();
            assert!(timestamp >= 0);
            timestamp
        })
    }
    pub fn enroll_until_timestamp(&self) -> Result<i64, ParseError> {
        Self::str_to_timestamp(&self.data.enrollment_until)
    }
    pub fn enroll_from_timestamp(&self) -> Result<i64, ParseError> {
        Self::str_to_timestamp(&self.data.enrollment_from)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub event_id: i64,
    #[serde(rename = "type")]
    pub type_field: i64,
    pub enrollment_enabled: bool,
    pub enrollment_from: String,
    pub enrollment_until: String,
    pub cancelation_until: String,
    pub lottery_duration: i64,
    pub starts: String,
    pub ends: String,
    pub cancellation_date: Value,
    pub cancellation_reason: Value,
    pub participants_min: Option<i64>,
    pub participants_max: i64,
    pub participant_count: i64,
    pub instructors: Vec<Instructor>,
    pub facilities: Vec<Facility>,
    pub rooms: Vec<String>,
    pub required_skills: Vec<Value>,
    pub sub_lessons: Vec<Value>,
    pub is_live_stream: bool,
    pub id: i64,
    pub base_type: i64,
    pub status: i64,
    pub number: String,
    pub sport_id: i64,
    pub sport_name: String,
    pub sport_url: String,
    pub title: String,
    pub location: Option<String>,
    pub web_registration_type: i64,
    pub meeting_point_info: Value,
    pub meeting_point_coordinates: Value,
    pub tl_comment_active: bool,
    pub tl_comment_active_info: bool,
    pub tl_comment: Value,
    pub language: Language,
    pub language_info: String,
    pub level_id: i64,
    pub level_info: String,
    pub level_e: bool,
    pub level_m: bool,
    pub level_f: bool,
    pub details: String,
    pub tl_tool_url: String,
    pub change_date: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instructor {
    pub asvz_id: i64,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Facility {
    pub facility_id: i64,
    pub name_short: String,
    pub name: String,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Language {
    pub id: String,
    pub code: String,
    pub name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonError {
    pub error_status: String,
    pub errors: Vec<Error>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    pub message: String,
}
