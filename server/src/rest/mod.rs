use std::{collections::HashMap, sync::Arc};

use axum::{
    extract::{FromRef, State},
    response::Html,
    routing::get,
    Json, Router,
};
use hackdose_sml_parser::application::{domain::AnyValue, obis::Obis};
use tokio::sync::Mutex;
use tower_http::services::ServeFile;

use crate::{data::EnergyData, Configuration};

use self::visualisation::render_image;

mod visualisation;

#[derive(Clone)]
struct SmartMeterState(Arc<Mutex<HashMap<Obis, AnyValue>>>);

#[derive(Clone)]
struct AppState {
    energy_data: EnergyData,
    smart_meter_state: SmartMeterState,
}

impl FromRef<AppState> for SmartMeterState {
    fn from_ref(input: &AppState) -> Self {
        input.smart_meter_state.clone()
    }
}

impl FromRef<AppState> for EnergyData {
    fn from_ref(input: &AppState) -> Self {
        input.energy_data.clone()
    }
}

pub(crate) async fn serve_rest_endpoint(
    mutex: Arc<Mutex<HashMap<Obis, AnyValue>>>,
    energy_data: EnergyData,
    config: &Configuration,
) {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    let app_state = AppState {
        energy_data: energy_data,
        smart_meter_state: SmartMeterState(mutex),
    };
    let app = Router::new()
        .route("/energy", get(return_energy))
        .route("/day", get(image))
        .nest_service("/log", ServeFile::new(config.log_location.clone()))
        .with_state(app_state);

    let _ = axum::serve(listener, app).await;
}

async fn return_energy(m: State<SmartMeterState>) -> Json<HashMap<Obis, AnyValue>> {
    let State(SmartMeterState(m)) = m;
    let res = &*m.lock().await;
    Json(res.clone())
}

async fn image(energy_data: State<EnergyData>) -> Html<String> {
    let State(e) = energy_data;
    let svg_image = render_image(e).await;
    Html(format!("<html>{}</html>", svg_image))
}
