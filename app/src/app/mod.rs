use crate::monitoring::Monitoring;
use crate::settings::Settings;
use patternfly_yew::prelude::*;
use yew::prelude::*;
use yew_nested_router::prelude::{Switch as RouterSwitch, *};
use yew_nested_router::Target;

#[derive(Debug, Default, Clone, PartialEq, Eq, Target)]
pub enum AppRoute {
    #[default]
    Settings,
    Monitoring,
}

#[function_component(Application)]
pub fn app() -> Html {
    html! {
        <ToastViewer>
            <Router<AppRoute> default={AppRoute::Settings}>
                <RouterSwitch<AppRoute> render={switch_app_route} />
            </Router<AppRoute>>
        </ToastViewer>
    }
}

fn switch_app_route(target: AppRoute) -> Html {
    match target {
        AppRoute::Settings => html! {<AppPage><Settings/></AppPage>},
        AppRoute::Monitoring => html! {<AppPage><Monitoring/></AppPage>},
    }
}

#[derive(Clone, Debug, PartialEq, Properties)]
pub struct PageProps {
    pub children: Children,
}

#[function_component(AppPage)]
fn page(props: &PageProps) -> Html {
    let sidebar = html_nested! {
        <PageSidebar>
                <Nav>
                    <NavList>
                        <NavRouterItem<AppRoute> to={AppRoute::Settings}>{"Settings"}</NavRouterItem<AppRoute>>
                        <NavRouterItem<AppRoute> to={AppRoute::Monitoring}>{"Monitoring"}</NavRouterItem<AppRoute>>
                    </NavList>
                </Nav>
        </PageSidebar>
    };

    let brand = html! (
        <MastheadBrand>
            <h1>{"Hackdose App"}</h1>
        </MastheadBrand>
    );

    html! (
        <Page {brand} {sidebar} >
            { for props.children.iter() }
        </Page>
    )
}
