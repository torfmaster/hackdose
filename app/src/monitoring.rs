use std::rc::Rc;

use crate::page::Page;
use patternfly_yew::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Request, RequestInit, RequestMode, Response};
use yew::prelude::*;
use yew_hooks::use_effect_once;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconDescriptor(Icon);

#[function_component(Monitoring)]
pub fn monitoring() -> Html {
    let image = use_state_eq::<Option<String>, _>(|| None);
    let de = (*image).clone();

    let svg = match &de {
        Some(d) => {
            let c = d.clone();
            Html::from_html_unchecked(AttrValue::from(c))
        }
        None => html! { "nothing" },
    };

    use_effect_once(|| {
        spawn_local(async move {
            let url = "/api/day_raw";
            let mut opts = RequestInit::new();
            opts.method("GET");
            opts.mode(RequestMode::Cors);

            let request = Request::new_with_str_and_init(url, &opts).unwrap();

            let window = gloo::utils::window();
            let resp_value = JsFuture::from(window.fetch_with_request(&request))
                .await
                .unwrap();
            let resp: Response = resp_value.dyn_into().unwrap();
            let text = JsFuture::from(resp.text().unwrap()).await.unwrap();
            let t = text.as_string().unwrap();
            image.clone().set(Some(t));
        });
        || ()
    });

    html!(
        <Page title="Monitoring" >
        {svg}
        </Page>
    )
}
