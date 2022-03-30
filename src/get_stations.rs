use crate::{Error, Gateway, Parameter, Station, BASE_URL};
use serde::Deserialize;

impl Gateway {
    pub async fn get_stations<'a>(&self, parameter: &'a Parameter) -> Result<Vec<Station>, Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Response {
            station: Vec<Station>,
        }

        //https://opendata-download-metobs.smhi.se/api/version/1.0/parameter/26.json

        let url = format!(
            "{}/parameter/{}.json",
            BASE_URL,
            serde_json::to_string(&parameter).unwrap()
        );
        let res: Response = self.get(&url).await?;
        Ok(res.station)
    }
}
