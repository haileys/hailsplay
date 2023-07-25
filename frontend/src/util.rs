use std::marker::PhantomData;

use web_sys::{AbortController, AbortSignal};
use futures::future::Future;
use yew::BaseComponent;

#[allow(unused)]
pub fn websocket_origin() -> String {
    let location = web_sys::window().unwrap().location();
    let proto = location.protocol().unwrap();
    let host = location.host().unwrap();

    // let host = host.replace(":8080", ":3000");

    let proto = match proto.as_str() {
        "https:" => "wss:",
        _ => "ws:",
    };

    format!("{}//{}", proto, host)
}

pub fn link<C: BaseComponent>(scope: &yew::html::Scope<C>) -> Link<C> {
    Link { scope: scope.clone() }
}

pub struct Link<C: BaseComponent> {
    scope: yew::html::Scope<C>,
}

impl<C: BaseComponent> Link<C> {
    pub fn map<T, F>(self, map: F) -> LinkMap<C, T,  F>
        where F: FnOnce(T) -> C::Message + 'static
    {
        let scope = self.scope;
        LinkMap { scope, map, _phantom: PhantomData }
    }
}

pub struct LinkMap<C: BaseComponent, T, F> {
    scope: yew::html::Scope<C>,
    map: F,
    _phantom: PhantomData<T>,
}

impl<C: BaseComponent, T, F> LinkMap<C, T, F>
    where F: FnOnce(T) -> C::Message + 'static
{
    pub fn spawn_cancellable<Fut>(
        self,
        func: impl FnOnce(AbortSignal) -> Fut + 'static,
    ) -> TaskHandle
        where Fut: Future<Output = T>,
    {
        let scope = self.scope;
        let map = self.map;

        let controller = AbortController::new()
            .expect("AbortController::new");
    
        let signal = controller.signal();

        wasm_bindgen_futures::spawn_local(async move {
            let result = func(signal.clone()).await;
            if !signal.aborted() {
                scope.send_message(map(result));
            }
        });
    
        TaskHandle { controller }
    }    
}

pub struct TaskHandle {
    controller: AbortController,
}

impl Drop for TaskHandle {
    fn drop(&mut self) {
        self.controller.abort();
    }
}
