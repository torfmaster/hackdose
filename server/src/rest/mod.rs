use std::{collections::HashMap, sync::Arc};

use hackdose_sml_parser::application::{domain::AnyValue, obis::Obis};
use tokio::sync::Mutex;
use warp::Filter;

use crate::data::EnergyData;

use self::visualisation::render_image;

mod visualisation;

pub(crate) async fn serve_rest_endpoint(
    mutex: Arc<Mutex<HashMap<Obis, AnyValue>>>,
    energy_data: EnergyData,
) {
    let energy = warp::path("energy")
        .map(move || mutex.clone())
        .and_then(return_energy);
    let image = warp::path("day")
        .and(warp::any().map(move || energy_data.clone()))
        .and_then(image);
    warp::serve(energy.or(image))
        .run(([0, 0, 0, 0], 8080))
        .await;
}

async fn return_energy(
    m: Arc<Mutex<HashMap<Obis, AnyValue>>>,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    Ok(Box::new(warp::reply::json(&*m.lock().await)))
}

async fn image(energy_data: EnergyData) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    let svg_image = render_image(energy_data).await;
    Ok(Box::new(warp::reply::html(format!(
        "<html>{}</html>",
        svg_image
    ))))
}
