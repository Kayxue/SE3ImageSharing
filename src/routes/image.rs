use std::{io::Cursor, path::PathBuf, sync::OnceLock};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::Path,
    http::{Response, StatusCode, header},
    middleware,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use axum_typed_multipart::{FieldData, TryFromMultipart, TypedMultipart};
use image::{EncodableLayout, ImageFormat, ImageReader};
use nanoid::nanoid;
use tokio::fs;

use tokio_util::io::ReaderStream;

use crate::middleware::checking_api_key;

static IMAGE_DIR: OnceLock<String> = OnceLock::new();

#[derive(TryFromMultipart)]
pub struct UploadImageForm {
    #[form_data(limit = "5MB")]
    pub image: FieldData<Bytes>,
}

async fn upload_image(TypedMultipart(body): TypedMultipart<UploadImageForm>) -> impl IntoResponse {
    let image_dir = IMAGE_DIR.get().expect("IMAGE_DIR not set");
    let image_name = nanoid!();

    match ImageReader::new(Cursor::new(body.image.contents.as_bytes())).with_guessed_format() {
        Ok(image) => match image.decode() {
            Ok(image) => {
                let mut cursor = Cursor::new(Vec::new());
                image.write_to(&mut cursor, ImageFormat::WebP).unwrap();
                fs::write(
                    format!("{}/{}.webp", image_dir, image_name),
                    cursor.into_inner(),
                )
                .await
                .unwrap();
                (axum::http::StatusCode::CREATED, image_name).into_response()
            }
            Err(e) => (
                axum::http::StatusCode::BAD_REQUEST,
                format!("Invalid image: {}", e),
            )
                .into_response(),
        },
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Cannot read image: {}", e),
        )
            .into_response(),
    }
}

async fn delete_image(Path(id): Path<String>) -> impl IntoResponse {
    let image_dir = IMAGE_DIR.get().expect("IMAGE_DIR not set");
    let image_path = format!("{}/{}.webp", image_dir, id);
    if PathBuf::from(&image_path).exists() {
        fs::remove_file(image_path).await.unwrap();
        (axum::http::StatusCode::OK, "Deleted").into_response()
    } else {
        (axum::http::StatusCode::NOT_FOUND, "Not found").into_response()
    }
}

async fn update_image(Path(id): Path<String>, TypedMultipart(body): TypedMultipart<UploadImageForm>) -> impl IntoResponse {
    let image_dir = IMAGE_DIR.get().expect("IMAGE_DIR not set");
    let image_path = format!("{}/{}.webp", image_dir, id);
    if !PathBuf::from(&image_path).exists() {
        return (axum::http::StatusCode::NOT_FOUND, "Not found").into_response();
    }
    match ImageReader::new(Cursor::new(body.image.contents.as_bytes())).with_guessed_format() {
        Ok(image) => match image.decode() {
            Ok(image) => {
                let mut cursor = Cursor::new(Vec::new());
                image.write_to(&mut cursor, ImageFormat::WebP).unwrap();
                fs::write(image_path, cursor.into_inner()).await.unwrap();
                (axum::http::StatusCode::OK, "Updated").into_response()
            }
            Err(e) => (
                axum::http::StatusCode::BAD_REQUEST,
                format!("Invalid image: {}", e),
            )
                .into_response(),
        },
        Err(e) => (
            axum::http::StatusCode::BAD_REQUEST,
            format!("Cannot read image: {}", e),
        )
            .into_response(),
    }
}

async fn get_image(Path(id): Path<String>) -> impl IntoResponse {
    let image_dir = IMAGE_DIR.get().expect("IMAGE_DIR not set");
    let image_path = format!("{}/{}.webp", image_dir, id);
    let file = match fs::File::open(image_path).await {
        Ok(file) => file,
        Err(e) => return (axum::http::StatusCode::NOT_FOUND, e.to_string()).into_response(),
    };

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/webp")
        .body(body)
        .unwrap()
}

async fn root() -> impl IntoResponse {
    "Hello, World!"
}

pub fn image_router(api_key: String, image_dir: String) -> Router {
    IMAGE_DIR.set(image_dir).expect("IMAGE_DIR already set");

    let api_key_required = Router::new()
        .route("/", post(upload_image))
        .route("/{id}", delete(delete_image))
        .route("/{id}", put(update_image))
        .route_layer(middleware::from_fn(checking_api_key(api_key)));

    Router::new()
        .route("/", get(root))
        .route("/{id}", get(get_image))
        .merge(api_key_required)
}
