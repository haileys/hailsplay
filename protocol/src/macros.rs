// this file is where all the grotty plumbing to
// generate our nice typescript types is kept

pub use tsify::Tsify;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// Url doesn't derive Tsify and so we don't get a typescript def for it:
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str =
    "export type Url = string;";

macro_rules! __type_attributes {
    () => {
    }
}

macro_rules! protocol {
    (
        $(#[$attrs:meta])*
        $vis:vis struct $name:ident { $($defs:tt)* }
        $($rest:tt)*
    ) => {
        $(#[$attrs])*
        #[cfg_attr(target_arch = "wasm32", derive(::tsify::Tsify))]
        #[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
        $vis struct $name { $($defs)* }

        protocol! { $($rest)* }
    };

    (
        $(#[$attrs:meta])*
        $vis:vis struct $name:ident ( $($defs:tt)* );
        $($rest:tt)*
    ) => {
        $(#[$attrs])*
        #[cfg_attr(target_arch = "wasm32", derive(::tsify::Tsify))]
        #[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
        $vis struct $name ( $($defs)* );

        protocol! { $($rest)* }
    };

    (
        $(#[$attrs:meta])*
        $vis:vis enum $name:ident { $($defs:tt)* }
        $($rest:tt)*
    ) => {
        $(#[$attrs])*
        #[cfg_attr(target_arch = "wasm32", derive(::tsify::Tsify))]
        #[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, from_wasm_abi))]
        $vis enum $name { $($defs)* }

        protocol! { $($rest)* }
    };

    () => {};
}
