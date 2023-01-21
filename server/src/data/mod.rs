use std::{collections::VecDeque, path::PathBuf, sync::Arc};

use chrono::{DateTime, Duration, Local, TimeZone};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    sync::Mutex,
};

use crate::Configuration;

pub(crate) type DataPoint = (DateTime<Local>, i32);

#[derive(Clone)]
pub(crate) struct EnergyData {
    store: Arc<Mutex<VecDeque<DataPoint>>>,
    log_location: PathBuf,
}

pub(crate) mod constants {
    pub(crate) const PERSIST_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
    pub(crate) const DATA_RETENTION_PERIOD_HOURS: i64 = 24;
}

impl EnergyData {
    pub(crate) async fn get_interval(
        &self,
        lower: DateTime<Local>,
        upper: DateTime<Local>,
    ) -> Vec<DataPoint> {
        let store = self.store.lock().await;
        store
            .iter()
            .cloned()
            .filter(|(t, _)| t >= &lower && t < &upper)
            .collect::<Vec<_>>()
    }

    pub(crate) async fn put(&mut self, data_point: DataPoint) {
        let mut store = self.store.lock().await;
        store.push_back(data_point);
        let old_index = store
            .iter()
            .enumerate()
            .find(|(_, e)| {
                data_point.0 - e.0 > Duration::hours(constants::DATA_RETENTION_PERIOD_HOURS)
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
        let to_date = Local::now();
        let from_date = to_date - Duration::hours(constants::DATA_RETENTION_PERIOD_HOURS);

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
                            let date = Local
                                .datetime_from_str(date, constants::PERSIST_DATE_FORMAT)
                                .unwrap();
                            let watts = i32::from_str_radix(value.as_str(), 10).unwrap();

                            if (date < to_date) && (date > from_date) {
                                buf.push_back((date, watts))
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
        let f = data.0.format(constants::PERSIST_DATE_FORMAT);
        let log_line = format!("{};{}\n", f, data.1);
        let log = tokio::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(self.log_location.clone())
            .await;

        dbg!(&log);
        dbg!(&self.log_location);
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
        let mut energy_data = EnergyData::default();
        let t_lower = Local.ymd(2022, 02, 02).and_hms(0, 0, 0);
        let t_upper = Local.ymd(2022, 04, 04).and_hms(0, 3, 0);
        let t_1 = Local.ymd(2022, 04, 04).and_hms(0, 1, 0);
        let t_2 = Local.ymd(2022, 04, 04).and_hms(0, 2, 0);

        let x_1 = 1;
        let x_2 = 2;

        energy_data.put((t_1, x_1)).await;
        energy_data.put((t_2, x_2)).await;

        let points = energy_data.get_interval(t_lower, t_upper).await;
        assert_eq!(points, vec![(t_1, x_1), (t_2, x_2)])
    }

    #[tokio::test]
    pub async fn omits_old_data_points() {
        let mut energy_data = EnergyData::default();
        let t_lower = Local.ymd(2022, 04, 04).and_hms(0, 1, 0);
        let t_upper = Local.ymd(2022, 04, 04).and_hms(0, 3, 0);

        let t_1 = Local.ymd(2022, 04, 04).and_hms(0, 0, 0);
        let t_2 = Local.ymd(2022, 04, 04).and_hms(0, 2, 0);
        let t_3 = Local.ymd(2022, 04, 04).and_hms(0, 4, 0);

        let x_1 = 1;
        let x_2 = 2;
        let x_3 = 3;

        energy_data.put((t_1, x_1)).await;
        energy_data.put((t_2, x_2)).await;
        energy_data.put((t_3, x_3)).await;

        let points = energy_data.get_interval(t_lower, t_upper).await;
        assert_eq!(points, vec![(t_2, x_2)])
    }

    #[tokio::test]
    pub async fn removes_old_data_points() {
        let mut energy_data = EnergyData::default();
        let t_lower = Local.ymd(2022, 04, 04).and_hms(11, 11, 11);
        let t_upper = Local.ymd(2022, 08, 04).and_hms(11, 11, 11);

        let t_1 = Local.ymd(2022, 07, 03).and_hms(11, 11, 11);
        let t_2 = Local.ymd(2022, 07, 05).and_hms(11, 11, 11);
        let t_3 = Local.ymd(2022, 07, 05).and_hms(11, 12, 11);

        let x_1 = 1;
        let x_2 = 2;
        let x_3 = 3;

        energy_data.put((t_1, x_1)).await;
        energy_data.put((t_2, x_2)).await;
        energy_data.put((t_3, x_3)).await;

        let points = energy_data.get_interval(t_lower, t_upper).await;
        assert_eq!(points, vec![(t_2, x_2), (t_3, x_3)])
    }
}
