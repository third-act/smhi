mod get_closest_station;
mod get_observations;
mod get_stations;
mod period;
pub use period::Period;
mod station;
pub use station::Station;
mod link;
pub use link::Link;
mod parameter;
pub use parameter::Parameter;
mod error;
pub use error::Error;
use serde::de::DeserializeOwned;
use std::time::Duration;
use tokio::time::sleep;

const BASE_URL: &str = "https://opendata-download-metobs.smhi.se/api/version/1.0";
const INITIAL_DELAY_MS: f64 = 100.0;
const RETRIES: u8 = 5;
const BACKOFF: f64 = 2.0;

pub struct Gateway {
    client: reqwest::Client,
}

impl Gateway {
    pub async fn new(timeout: Option<Duration>) -> Result<Gateway, Error> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Content-Type",
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.insert(
            "Accept",
            reqwest::header::HeaderValue::from_static("application/json"),
        );

        let timeout = match timeout {
            Some(t) => t,
            None => Duration::new(60, 0),
        };

        let client = match reqwest::ClientBuilder::new()
            .default_headers(headers)
            .https_only(true)
            .timeout(timeout)
            .build()
        {
            Ok(r) => r,
            Err(err) => {
                return Err(Error::Unspecified(format!(
                    "Could not create reqwest client ({}).",
                    err.to_string()
                )))
            }
        };

        let c = Gateway { client };
        Ok(c)
    }

    async fn get<'a, T: DeserializeOwned>(&self, url: &str) -> Result<T, Error> {
        let mut delay = INITIAL_DELAY_MS;
        for _ in 0..RETRIES {
            let text = match self.get_without_retry(url).await {
                Ok(res) => res,
                Err(err) => {
                    delay = self.randomized_exponential_backoff(delay).await;

                    match err {
                        Error::Throttling => {
                            continue;
                        }
                        _ => return Err(err),
                    };
                }
            };

            let body: T = match serde_json::from_str(&text) {
                Ok(r) => r,
                Err(err) => {
                    return Err(Error::SerializationError(format!(
                        "Could not deserialize response from \"{}\" ({}).",
                        text,
                        err.to_string()
                    )))
                }
            };

            return Ok(body);
        }

        Err(Error::Throttling)
    }

    async fn get_string<'a>(&self, url: &str) -> Result<String, Error> {
        let mut delay = INITIAL_DELAY_MS;
        for _ in 0..RETRIES {
            let text = match self.get_without_retry(url).await {
                Ok(res) => res,
                Err(err) => {
                    delay = self.randomized_exponential_backoff(delay).await;

                    match err {
                        Error::Throttling => {
                            continue;
                        }
                        _ => return Err(err),
                    };
                }
            };

            return Ok(text);
        }

        Err(Error::Throttling)
    }

    async fn get_without_retry<'a>(&self, url: &str) -> Result<String, Error> {
        let res = match self.client.get(url).send().await {
            Ok(r) => r,
            Err(err) => {
                return Err(Error::NetworkError(format!(
                    "Could not send request ({}).",
                    err.to_string()
                )))
            }
        };

        let status = res.status().as_u16();
        let text = res
            .text()
            .await
            .unwrap_or_else(|_| String::from("Could not retrieve body text."));

        if status < 200 || status > 299 {
            if status == 429 {
                return Err(Error::Throttling);
            }

            return Err(Error::ApiError(status, text));
        }

        Ok(text)
    }

    // Randomized exponential backoff policy (cf.
    // https://cloud.google.com/appengine/articles/scalability#backoff ).
    async fn randomized_exponential_backoff(&self, mut delay_ms: f64) -> f64 {
        //let mut rng = rand::thread_rng();

        // Random component to avoid thundering herd problem (values taken from
        // https://github.com/GoogleCloudPlatform/appengine-gcs-client/blob/master/java/src/main/
        // java/com/google/appengine/tools/cloudstorage/RetryHelper.java ).
        //delay_ms = (rng.gen::<f64>() / 2.0 + 0.75) * delay_ms;

        sleep(Duration::from_millis(delay_ms as u64)).await;

        delay_ms *= BACKOFF;
        delay_ms
    }
}
