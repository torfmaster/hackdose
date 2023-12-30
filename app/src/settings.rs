use crate::page::Page;
use patternfly_yew::prelude::*;
use yew::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IconDescriptor(Icon);

#[function_component(Settings)]
pub fn icons() -> Html {
    html!(
        <Page title="Settings" >
        {"This page will contain the Settings"}

        </Page>
    )
}
