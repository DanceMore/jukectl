use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use std::fmt;

//use crate::HashableSong;
//use crate::MpdConn;

// TagData, useful holder for any_tags vs not_tags
#[derive(Clone, Serialize, Deserialize)]
pub struct TagsData {
    pub any: Vec<String>,
    pub not: Vec<String>,
}

impl TagsData {
    fn tags_to_strings(&self) -> (HashSet<String>, HashSet<String>) {
        let any_tags: HashSet<String> = self
            .any
            .iter()
            .flat_map(|s| s.split(',').map(String::from))
            .collect();
        let not_tags: HashSet<String> = self
            .not
            .iter()
            .flat_map(|s| s.split(',').map(String::from))
            .collect();

        (any_tags, not_tags)
    }
}


// Implement the Debug trait for TagsData
impl<> fmt::Debug for TagsData<> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{\n    any: {:?},\n    not: {:?}\n}}", self.any, self.not)
    }
}

impl<> TagsData<> {
    fn to_json(&self) -> String {
        match serde_json::to_string(self) {
            Ok(json) => json,
            Err(err) => {
                eprintln!("[!!!] Error serializing TagsData to JSON: {}", err);
                std::process::exit(1); // Exit with a non-zero status code
            }
        }
    }
}
