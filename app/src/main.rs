#![recursion_limit = "1024"]

mod app;
mod monitoring;
mod page;
mod settings;

use wasm_bindgen::prelude::*;

pub fn main() -> Result<(), JsValue> {
    yew::Renderer::<app::Application>::new().render();
    Ok(())
}
