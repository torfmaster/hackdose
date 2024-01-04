use patternfly_yew::prelude::*;
use yew::prelude::*;

#[derive(Clone, Debug, Properties, PartialEq)]
pub struct Props {
    pub title: AttrValue,
    #[prop_or_default]
    pub experimental: bool,
    #[prop_or_default]
    pub subtitle: Children,
    #[prop_or_default]
    pub children: Children,
}

#[function_component(Page)]
pub fn page(props: &Props) -> Html {
    html! (
        <PageSectionGroup>
            <PageSection
                r#type={PageSectionType::Default}
                variant={PageSectionVariant::Light}
                limit_width=true
                sticky={[PageSectionSticky::Top]}
            >
                <Content>
                    <Title size={Size::XXLarge}>
                        { props.title.clone() }
                    </Title>
                    { for props.subtitle.iter() }
                </Content>
            </PageSection>


            { for props.children.iter().map(|child|{
                html!(<PageSection>{child}</PageSection>)
            })}
        </PageSectionGroup>
    )
}
