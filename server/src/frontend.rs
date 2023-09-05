pub fn serve(router: axum::Router) -> axum::Router {
    imp::serve(router)
}

#[cfg(not(feature = "bundle"))]
mod imp {
    pub fn serve(router: axum::Router) -> axum::Router {
        router
    }
}

#[cfg(feature = "bundle")]
mod imp {
    use axum::{response::IntoResponse, routing::get};

    pub fn serve(router: axum::Router) -> axum::Router {
        router
            .route("/", get(index_html))
            .route(env!("BUNDLE_INDEX_CSS_URL"), get(index_css))
            .route(env!("BUNDLE_INDEX_JS_URL"), get(index_js))
    }

    async fn index_html() -> impl IntoResponse {
        ([("content-type", "text/html")],
            include_str!(env!("BUNDLE_INDEX_HTML_PATH")))
    }

    async fn index_css() -> impl IntoResponse {
        ([("content-type", "text/css")],
            include_str!(env!("BUNDLE_INDEX_CSS_PATH")))
    }

    async fn index_js() -> impl IntoResponse {
        ([("content-type", "application/javascript")],
            include_str!(env!("BUNDLE_INDEX_JS_PATH")))
    }
}
