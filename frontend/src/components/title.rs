use yew::prelude::*;

#[function_component(Title)]
pub fn title() -> Html {
    html! {
        <div class="title-stack">
            <div class="header">
                <div class="app-name">{"hailsPlay"}</div>
            </div>
            <AddInput />
        </div>
    }
}
