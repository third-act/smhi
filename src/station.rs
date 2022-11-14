use crate::Link;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Station {
    pub id: u32,

    pub name: String,

    pub latitude: f64,

    pub longitude: f64,

    pub active: bool,

    pub title: String,

    pub summary: String,

    pub link: Vec<Link>,
}
