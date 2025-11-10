use axum::{
    extract::Request,
    http::HeaderMap,
    middleware::Next,
    response::{IntoResponse, Response},
};

pub fn checking_api_key(
    api_key: String,
) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> + Clone {
    move |req: Request, next: Next| {
        let api_key = api_key.clone();
        Box::pin(async move {
            let headers: &HeaderMap = req.headers();
            match headers.get("key") {
                Some(key) if key == api_key.as_str() => next.run(req).await,
                _ => {
                    (
                        axum::http::StatusCode::UNAUTHORIZED,
                        "Unauthorized: Invalid or missing API key",
                    )
                        .into_response()
                }
            }
        })
    }
}