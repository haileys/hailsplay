mod app;
mod components;
mod util;
mod subscribe;

use app::App;

fn main() {
    // session::start();
    yew::Renderer::<App>::new().render();
}

#[macro_export]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}
