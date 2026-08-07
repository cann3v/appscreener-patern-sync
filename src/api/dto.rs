use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PatternType {
    Reporting,
    Dataflow,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueryType {
    Regex,
    Xpath,
}

/// Ответ appScreener.
///
/// Согласно актуальной OpenAPI-схеме обязательными полями являются только
/// `name` и `xml`. Остальные поля могут отсутствовать в ответах разных версий.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<i32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<i32>,

    pub name: String,

    pub xml: String,

    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub pattern_type: Option<PatternType>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared: Option<bool>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_type: Option<QueryType>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_regex: Option<String>,
}

/// Тело POST/PUT.
///
/// Серверные поля `shared` и `user` намеренно не отправляются.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PatternWrite {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,

    pub rule_id: String,

    pub severity: i32,

    pub confidence: i32,

    pub name: String,

    pub xml: String,

    #[serde(rename = "type")]
    pub pattern_type: PatternType,

    pub active: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_type: Option<QueryType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_regex: Option<String>,
}
