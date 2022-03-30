use crate::{Error, Gateway, Link, Parameter, Period, BASE_URL};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use serde::Deserialize;

impl Gateway {
    pub async fn get_observations<'a>(
        &self,
        station_id: u32,
        parameter: &'a Parameter,
    ) -> Result<Vec<(NaiveDateTime, f64)>, Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct PeriodResponse {
            period: Vec<Period>,
        }

        let parameter_id = serde_json::to_string(&parameter).unwrap();

        //https://opendata-download-metobs.smhi.se/api/version/1.0/parameter/26/station/
        //188790/period.json

        let url = format!(
            "{}/parameter/{}/station/{}/period.json",
            BASE_URL, parameter_id, station_id
        );
        let res: PeriodResponse = self.get(&url).await?;

        let mut corrected_archive_url = None;
        let mut latest_months_url = None;
        for period in res.period {
            if period.key == "corrected-archive" {
                for link in period.link {
                    if link.r#type == "application/json" {
                        corrected_archive_url = Some(link.href);
                        break;
                    }
                }
            } else if period.key == "latest-months" {
                for link in period.link {
                    if link.r#type == "application/json" {
                        latest_months_url = Some(link.href);
                        break;
                    }
                }
            }

            if let (Some(_), Some(_)) = (&corrected_archive_url, &latest_months_url) {
                break;
            }
        }
        let corrected_archive_url = match corrected_archive_url {
            Some(corrected_archive_url) => corrected_archive_url,
            None => {
                return Err(Error::ParseError(format!(
                    "Could not find corrected-archive in period."
                )));
            }
        };
        let latest_months_url = match latest_months_url {
            Some(latest_months_url) => latest_months_url,
            None => {
                return Err(Error::ParseError(format!(
                    "Could not find latest-months in period."
                )));
            }
        };

        // Get corrected archive.

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CorrectedArchiveData {
            link: Vec<Link>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CorrectedArchiveResponse {
            data: Vec<CorrectedArchiveData>,
        }

        //https://opendata-download-metobs.smhi.se/api/version/1.0/parameter/26/station/188790/
        //period/corrected-archive.json

        let res: CorrectedArchiveResponse = self.get(&corrected_archive_url).await?;

        let mut corrected_archive_data_url = None;
        'off: for data in res.data {
            for link in data.link {
                if link.rel == "data" {
                    corrected_archive_data_url = Some(link.href);
                    break 'off;
                }
            }
        }
        let corrected_archive_data_url = match corrected_archive_data_url {
            Some(corrected_archive_data_url) => corrected_archive_data_url,
            None => {
                return Err(Error::ParseError(format!(
                    "Could not find data in corrected-archive."
                )));
            }
        };

        // Get csv data.

        //https://opendata-download-metobs.smhi.se/api/version/1.0/parameter/26/station/188790/
        //period/corrected-archive/data.csv

        let corrected_archive_data = self.get_string(&corrected_archive_data_url).await?;

        let mut observations = vec![];

        // Parse corrected archive data.
        let rows = corrected_archive_data.split("\n").collect::<Vec<&str>>();
        for row in rows {
            if row == "" {
                continue;
            }

            let columns = row.split(";").collect::<Vec<&str>>();

            //1961-01-01;18:00:00;-0.2;G

            // We will accept a row as an observation if
            // 1) a row has minimum 4 columns,
            if columns.len() < 4 {
                continue;
            }

            // 2) a valid naive date is found in column one,
            let date = match NaiveDate::parse_from_str(columns[0], "%Y-%m-%d") {
                Ok(date) => date,
                Err(_) => {
                    println!("Error parsing {} as date.", columns[0]);
                    continue;
                }
            };

            // 3) a valid time is found in column two,
            let time = match NaiveTime::parse_from_str(columns[1], "%H:%M:%S") {
                Ok(time) => time,
                Err(_) => {
                    println!("Error parsing {} as time.", columns[1]);
                    continue;
                }
            };

            // 4) a number is found in column three, and
            let value = match columns[2].parse::<f64>() {
                Ok(value) => value,
                Err(_) => {
                    println!("Error parsing {} as a number.", columns[2]);
                    continue;
                }
            };

            // 5) either a G or a Y is found in column four.
            if columns[3] != "G" && columns[3] != "Y" {
                continue;
            }

            observations.push((date.and_time(time), value));
        }

        // Get latest months.

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct LatestMonthsData {
            link: Vec<Link>,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct LatestMonthsResponse {
            data: Vec<LatestMonthsData>,
        }

        let res: LatestMonthsResponse = self.get(&latest_months_url).await?;

        let mut latest_months_data_url = None;
        'out: for data in res.data {
            for link in data.link {
                if link.r#type == "application/json" {
                    latest_months_data_url = Some(link.href);
                    break 'out;
                }
            }
        }
        let latest_months_data_url = match latest_months_data_url {
            Some(latest_months_data_url) => latest_months_data_url,
            None => {
                return Err(Error::ParseError(format!(
                    "Could not find json data in latest-months."
                )));
            }
        };

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Value {
            date: u64,
            value: String,
            quality: String,
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct LatestMonthsDataResponse {
            value: Vec<Value>,
        }

        let res: LatestMonthsDataResponse = self.get(&latest_months_data_url).await?;

        // Add the last four months. NOTE: We do not handle duplicates, as the client may not want
        // the performance penalty.
        for data in res.value {
            // NOTE: The timestamp is in milliseconds.
            let timestamp = (data.date as f64 / 1000.0) as i64;
            let date = match NaiveDateTime::from_timestamp_opt(timestamp, 0) {
                Some(date) => date,
                None => {
                    println!("Error parsing {} as date.", timestamp);
                    continue;
                }
            };

            println!("XXX {} {}", data.date, date);

            let value = match data.value.parse::<f64>() {
                Ok(value) => value,
                Err(_) => {
                    println!("Error parsing {} as a number.", data.value);
                    continue;
                }
            };

            if data.quality != "G" && data.quality != "Y" {
                continue;
            }

            observations.push((date, value));
        }

        Ok(observations)
    }
}
