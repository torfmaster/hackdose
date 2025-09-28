use reqwest::Url;

use super::PowerSwitch;

pub(crate) struct TasmotaSwitch {
    pub(crate) url: String,
}

#[async_trait::async_trait]
impl PowerSwitch for TasmotaSwitch {
    async fn on(&mut self) {
        let mut url = Url::parse(&self.url).unwrap().join("cm").unwrap();
        url.query_pairs_mut().append_pair("cmnd", "Power on");
        let _ = reqwest::get(url).await;
    }

    async fn off(&mut self) {
        let mut url = Url::parse(&self.url).unwrap().join("cm").unwrap();
        url.query_pairs_mut().append_pair("cmnd", "Power off");
        let _ = reqwest::get(url).await;
    }
}
