use crate::{Error, Gateway, Parameter, Station, BASE_URL};
use serde::Deserialize;

impl Gateway {
    pub async fn get_closest_station<'a>(
        &self,
        latitude: f64,
        longitude: f64,
        parameter: &'a Parameter,
    ) -> Result<(Station, f64), Error> {
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

        // Find closest.
        let mut closest = None;
        for station in res.station {
            // NOTE: We are using flat earth approximation, as the distances are expected to be \
            // small, otherwise the data from the station will not be relevant anyway.
            let distance =
                distance_meters(latitude, longitude, station.latitude, station.longitude);
            match &closest {
                Some((_, closest_distance)) => {
                    if &distance < closest_distance {
                        closest = Some((station.clone(), distance));
                    }
                }
                None => closest = Some((station.clone(), distance)),
            }
        }

        match closest {
            Some((station, distance)) => Ok((station, distance)),
            None => Err(Error::NotFound),
        }
    }
}

fn distance_meters(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let x = deg2rad(lon1 - lon2) * f64::cos(deg2rad((lat1 + lat2) / 2.0));
    let y = deg2rad(lat1 - lat2);
    let dist = 6371000.0 * f64::sqrt(x * x + y * y);
    return dist;
}

fn deg2rad(degrees: f64) -> f64 {
    let pi = std::f64::consts::PI;
    return degrees * (pi / 180.0);
}
