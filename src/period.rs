use crate::Link;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Period {
    pub key: String,

    pub title: String,

    pub summary: String,

    pub link: Vec<Link>,
}
