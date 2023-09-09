use rocket::data::ByteUnit;
use rocket::data::{FromData, Outcome};
use rocket::http::Status;
use rocket::{Data, Request, State};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagsData {
    pub any: Vec<String>,
    pub not: Vec<String>,
}
