use crate::{
    monitoring::{utils::get_viewport, visualization::render_image},
    page::Page,
};
use chrono::{Duration, Utc};
use hackdose_server_shared::{PlotData, Range};
use patternfly_yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Request, RequestInit, RequestMode, Response};
use yew::prelude::*;

mod utils;
mod visualization;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconDescriptor(Icon);

#[function_component(Monitoring)]
pub fn monitoring() -> Html {
    let image = use_state_eq::<Option<String>, _>(|| None);

    let offset = use_state_eq(|| OffsetSelection::Hour);
    let de = (*image).clone();

    let svg = match &de {
        Some(d) => {
            let c = d.clone();
            Html::from_html_unchecked(AttrValue::from(c))
        }
        None => html! { "nothing" },
    };
    let offset4 = offset.clone();

    use_effect_with(offset.clone(), |_| {
        spawn_local(async move {
            let new_from = Utc::now() - (*offset4).to_duration();
            let new_to = Utc::now();

            let from_millis = new_from.timestamp_millis();
            let to_millis = new_to.timestamp_millis();

            let url = format!("/api/data_raw?from={from_millis}&to={to_millis}");
            let mut opts = RequestInit::new();
            opts.method("GET");
            opts.mode(RequestMode::Cors);

            let request = Request::new_with_str_and_init(&url, &opts).unwrap();

            let window = gloo::utils::window();
            let resp_value = JsFuture::from(window.fetch_with_request(&request))
                .await
                .unwrap();
            let resp: Response = resp_value.dyn_into().unwrap();
            let text = JsFuture::from(resp.text().unwrap()).await.unwrap();
            let t = text.as_string().unwrap();
            let image_data = serde_json::from_str::<PlotData>(&t).unwrap();

            let (width, height) = get_viewport();

            let svg = render_image(
                &image_data,
                Range {
                    from: new_from,
                    to: new_to,
                },
                width,
                height,
            );
            image.set(Some(svg));
        });
        || ()
    });
    let offset2 = offset.clone();
    let offset5 = offset.clone();

    html!(
        <Page title="Monitoring" >
        <Dropdown
        text={offset5.to_caption()}
    >
        <MenuAction onclick={move |()| offset.set(OffsetSelection::Hour)}>{OffsetSelection::Hour.to_caption()}</MenuAction>
        <ListDivider/>
        <MenuAction onclick={move |()| offset2.set(OffsetSelection::Day)}>{OffsetSelection::Day.to_caption()}</MenuAction>
        <ListDivider/>
        <MenuAction onclick={move |()| offset5.set(OffsetSelection::Week)}>{OffsetSelection::Week.to_caption()}</MenuAction>
    </Dropdown>
        {svg}
        </Page>
    )
}

#[derive(PartialEq, Eq, Debug)]
enum OffsetSelection {
    Hour,
    Day,
    Week,
}

impl OffsetSelection {
    fn to_duration(&self) -> Duration {
        match self {
            OffsetSelection::Hour => Duration::hours(1),
            OffsetSelection::Day => Duration::days(1),
            OffsetSelection::Week => Duration::weeks(1),
        }
    }

    fn to_caption(&self) -> String {
        match self {
            OffsetSelection::Hour => "1 Hour".into(),
            OffsetSelection::Day => "1 Day".into(),
            OffsetSelection::Week => "1 Week".into(),
        }
    }
}
