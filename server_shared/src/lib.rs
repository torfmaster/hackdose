use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct DataPoint {
    #[serde(with = "ts_milliseconds")]
    pub date: DateTime<Utc>,
    pub value: i32,
}

impl DataPoint {
    pub fn to_tuple(&self) -> (DateTime<Utc>, i32) {
        (self.date, self.value)
    }

    fn from_tuple(tuple: &(DateTime<Utc>, i32)) -> Self {
        DataPoint {
            date: tuple.0,
            value: tuple.1,
        }
    }
}

pub type PlotData = Vec<DataPoint>;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]

pub struct Range {
    #[serde(with = "ts_milliseconds")]
    pub from: DateTime<Utc>,
    #[serde(with = "ts_milliseconds")]
    pub to: DateTime<Utc>,
}
