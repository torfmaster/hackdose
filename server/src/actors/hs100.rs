use tokio::task::spawn_blocking;
use tplinker::capabilities::Switch;
use tplinker::devices::HS100;

use super::PowerSwitch;

pub(crate) struct HS100Switch {
    pub(crate) address: String,
}

#[async_trait::async_trait]
impl PowerSwitch for HS100Switch {
    async fn on(&mut self) {
        let address = self.address.clone();
        spawn_blocking(move || {
            let dev = HS100::new(&address);
            if let Ok(dev) = dev {
                let _ = dev.switch_on();
            }
        })
        .await
        .unwrap();
    }

    async fn off(&mut self) {
        let address = self.address.clone();
        spawn_blocking(move || {
            let dev = HS100::new(&address);
            if let Ok(dev) = dev {
                let _ = dev.switch_off();
            }
        })
        .await
        .unwrap();
    }

    async fn set_power(&mut self, _: isize) {}
}
