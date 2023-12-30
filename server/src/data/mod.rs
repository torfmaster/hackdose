use std::{collections::VecDeque, path::PathBuf, sync::Arc};

use chrono::{DateTime, Duration, TimeZone, Utc};
use hackdose_server_shared::DataPoint;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::Mutex,
};

use crate::Configuration;

#[derive(Clone)]
pub(crate) struct EnergyData {
    store: Arc<Mutex<VecDeque<DataPoint>>>,
    log_location: PathBuf,
}

pub(crate) mod constants {
    pub(crate) const PERSIST_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
    pub(crate) const DATA_RETENTION_PERIOD_DAYS: i64 = 7;
}

impl EnergyData {
    pub(crate) async fn get_interval(
        &self,
        lower: DateTime<Utc>,
        upper: DateTime<Utc>,
    ) -> Vec<DataPoint> {
        let store = self.store.lock().await;
        store
            .iter()
            .map(|x| *x)
            .filter(|p| p.date >= lower && p.date < upper)
            .collect::<Vec<_>>()
    }

    pub(crate) async fn put(&mut self, data_point: DataPoint) {
        let mut store = self.store.lock().await;
        store.push_back(data_point);
        let old_index = store
            .iter()
            .enumerate()
            .find(|(_, e)| {
                data_point.date - e.date > Duration::days(constants::DATA_RETENTION_PERIOD_DAYS)
            })
            .map(|(i, _)| i);
        match old_index {
            Some(old_index) => {
                store.drain(0..=old_index);
            }
            None => (),
        }
    }

    pub(crate) async fn try_from_file(config: &Configuration) -> Self {
        let to_date = Utc::now();
        let from_date = to_date - Duration::days(constants::DATA_RETENTION_PERIOD_DAYS);

        let energy_data = EnergyData {
            store: Default::default(),
            log_location: config.log_location.clone(),
        };
        let mut buf = energy_data.store.lock().await;
        let data = File::open(config.log_location.clone()).await;
        match data {
            Ok(data) => {
                let mut rdr = BufReader::new(data).lines();
                while let Ok(Some(line)) = rdr.next_line().await {
                    let l = line
                        .split(";")
                        .map(|x| x.to_string())
                        .collect::<Vec<String>>();
                    match &l[..] {
                        [date, value, ..] => {
                            let date = Utc.datetime_from_str(date, constants::PERSIST_DATE_FORMAT);
                            match date {
                                Ok(date) => {
                                    let watts = i32::from_str_radix(value.as_str(), 10).unwrap();

                                    if (date < to_date) && (date > from_date) {
                                        buf.push_back(DataPoint {
                                            date: date.into(),
                                            value: watts,
                                        })
                                    }
                                }
                                Err(_) => (),
                            }
                        }
                        _ => (),
                    }
                }
                drop(buf);
                energy_data
            }
            Err(_) => Self {
                store: Default::default(),
                log_location: config.log_location.clone(),
            },
        }
    }

    pub(crate) async fn log_data(&self, data: DataPoint) {
        let f = data.date.format(constants::PERSIST_DATE_FORMAT);
        let log_line = format!("{};{}\n", f, data.value);
        let log = tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(self.log_location.clone())
            .await;

        match log {
            Ok(mut file) => {
                let _ = file.write_all(log_line.as_bytes()).await;
            }
            Err(_) => (),
        }
    }
}

#[cfg(test)]
mod test {
    use chrono::TimeZone;

    use super::*;

    #[tokio::test]
    pub async fn gets_data_points() {
        let mut energy_data = EnergyData {
            store: Default::default(),
            log_location: Default::default(),
        };
        let t_lower = Utc.with_ymd_and_hms(2022, 02, 02, 0, 0, 0).unwrap();
        let t_upper = Utc.with_ymd_and_hms(2022, 04, 04, 0, 3, 0).unwrap();
        let t_1 = Utc.with_ymd_and_hms(2022, 04, 04, 0, 1, 0).unwrap();
        let t_2 = Utc.with_ymd_and_hms(2022, 04, 04, 0, 2, 0).unwrap();

        let x_1 = 1;
        let x_2 = 2;

        energy_data
            .put(DataPoint {
                date: t_1,
                value: x_1,
            })
            .await;
        energy_data
            .put(DataPoint {
                date: t_2,
                value: x_2,
            })
            .await;

        let points = energy_data.get_interval(t_lower, t_upper).await;
        assert_eq!(
            points,
            vec![
                DataPoint {
                    date: t_1,
                    value: x_1
                },
                DataPoint {
                    date: t_2,
                    value: x_2
                }
            ]
        )
    }

    #[tokio::test]
    pub async fn omits_old_data_points() {
        let mut energy_data = EnergyData {
            store: Default::default(),
            log_location: Default::default(),
        };
        let t_lower = Utc.with_ymd_and_hms(2022, 04, 04, 0, 1, 0).unwrap();
        let t_upper = Utc.with_ymd_and_hms(2022, 04, 04, 0, 3, 0).unwrap();

        let t_1 = Utc.with_ymd_and_hms(2022, 04, 04, 0, 0, 0).unwrap();
        let t_2 = Utc.with_ymd_and_hms(2022, 04, 04, 0, 2, 0).unwrap();
        let t_3 = Utc.with_ymd_and_hms(2022, 04, 04, 0, 4, 0).unwrap();

        let x_1 = 1;
        let x_2 = 2;
        let x_3 = 3;

        energy_data
            .put(DataPoint {
                date: t_1,
                value: x_1,
            })
            .await;
        energy_data
            .put(DataPoint {
                date: t_2,
                value: x_2,
            })
            .await;
        energy_data
            .put(DataPoint {
                date: t_3,
                value: x_3,
            })
            .await;

        let points = energy_data.get_interval(t_lower, t_upper).await;
        assert_eq!(
            points,
            vec![DataPoint {
                date: t_2,
                value: x_2
            }]
        )
    }

    #[tokio::test]
    pub async fn removes_old_data_points() {
        let mut energy_data = EnergyData {
            store: Default::default(),
            log_location: Default::default(),
        };
        let t_lower = Utc.with_ymd_and_hms(2022, 04, 04, 11, 11, 11).unwrap();
        let t_upper = Utc.with_ymd_and_hms(2022, 08, 04, 11, 11, 11).unwrap();

        let t_1 = Utc.with_ymd_and_hms(2022, 06, 10, 11, 11, 11).unwrap();
        let t_2 = Utc.with_ymd_and_hms(2022, 07, 05, 11, 11, 11).unwrap();
        let t_3 = Utc.with_ymd_and_hms(2022, 07, 05, 11, 12, 11).unwrap();

        let x_1 = 1;
        let x_2 = 2;
        let x_3 = 3;

        energy_data
            .put(DataPoint {
                date: t_1,
                value: x_1,
            })
            .await;
        energy_data
            .put(DataPoint {
                date: t_2,
                value: x_2,
            })
            .await;
        energy_data
            .put(DataPoint {
                date: t_3,
                value: x_3,
            })
            .await;

        let points = energy_data.get_interval(t_lower, t_upper).await;
        assert_eq!(
            points,
            vec![
                DataPoint {
                    date: t_2,
                    value: x_2
                },
                DataPoint {
                    date: t_3,
                    value: x_3
                }
            ]
        )
    }
}
