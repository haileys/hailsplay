use yew::prelude::*;

use crate::components::Add;

#[function_component(App)]
pub fn app() -> Html {
    html! {
        <>
            <header>
                <div class="app-name">{"hailsPlay"}</div>
            </header>
            <main>
                <div class="main-border main-border-top"></div>
                <div class="main-content"></div>
                <div class="main-border main-border-bottom"></div>
            </main>
            <Add />
        </>
    }
}
