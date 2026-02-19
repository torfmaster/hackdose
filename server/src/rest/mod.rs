use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Body,
    extract::{FromRef, Path, Query, State},
    http::{header, HeaderValue, Response, StatusCode},
    routing::get,
    Json, Router,
};
use hackdose_server_shared::{DataPoint, Range};
use hackdose_sml_parser::application::{domain::AnyValue, obis::Obis};
use mime_guess::mime;
use tokio::sync::Mutex;
use tower_http::{compression::CompressionLayer, cors::CorsLayer, services::ServeFile};

use crate::{data::EnergyData, Configuration};

use include_dir::{include_dir, Dir};

static STATIC_DIR: Dir<'_> = include_dir!("app/dist/");

async fn static_path(Path(path): Path<String>) -> Response<Body> {
    let path = path.trim_start_matches('/');
    let path = if path == "" { "index.html" } else { path };
    let mime_type = mime_guess::from_path(path).first_or_text_plain();

    match STATIC_DIR.get_file(path) {
        None => index_response(),
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref()).unwrap(),
            )
            .body(Body::from(file.contents()))
            .unwrap(),
    }
}

fn index_response() -> Response<Body> {
    let file = STATIC_DIR.get_file("index.html").unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_str(mime::TEXT_HTML_UTF_8.as_ref()).unwrap(),
        )
        .body(Body::from(file.contents()))
        .unwrap()
}

async fn index_handler() -> Response<Body> {
    index_response()
}

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
        .route("/api/energy", get(return_energy))
        .route("/api/data_raw", get(data_raw))
        .layer(CorsLayer::permissive())
        .nest_service("/api/log", ServeFile::new(config.log_location.clone()))
        .layer(CompressionLayer::new().deflate(true))
        .route("/*path", get(static_path))
        .fallback(index_handler)
        .with_state(app_state);

    let _ = axum::serve(listener, app).await;
}

async fn return_energy(m: State<SmartMeterState>) -> Json<HashMap<Obis, AnyValue>> {
    let State(SmartMeterState(m)) = m;
    let res = &*m.lock().await;
    Json(res.clone())
}

async fn data_raw(energy_data: State<EnergyData>, range: Query<Range>) -> Json<Vec<DataPoint>> {
    let State(e) = energy_data;

    let data = e.get_interval(range.from, range.to).await.clone();
    Json(data)
}
