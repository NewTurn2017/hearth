use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Project {
    pub id: i64,
    pub priority: String,
    pub number: Option<i64>,
    pub name: String,
    pub category: Option<String>,
    pub path: Option<String>,
    pub evaluation: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Schedule {
    pub id: i64,
    pub date: String,
    pub time: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub notes: Option<String>,
    pub remind_before_5min: bool,
    pub remind_at_start: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Memo {
    pub id: i64,
    pub content: String,
    pub color: String,
    pub project_id: Option<i64>,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Client {
    pub id: i64,
    pub company_name: Option<String>,
    pub ceo: Option<String>,
    pub phone: Option<String>,
    pub fax: Option<String>,
    pub email: Option<String>,
    pub offices: Option<String>,
    pub project_desc: Option<String>,
    pub status: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
