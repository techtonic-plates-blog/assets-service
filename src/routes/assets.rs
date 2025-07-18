use crate::auth::BearerAuthorization;
use crate::connections::ObjectStorage;
use crate::connections::object_storage::ASSETS_FILE_BUCKET;
use crate::routes::ApiTags;
use bytes::Bytes;
use minio::s3::segmented_bytes::SegmentedBytes;
use minio::s3::types::{S3Api, ToStream};
use poem::Error;
use poem::http::StatusCode;
use poem::{Result, error::InternalServerError, web::Data};
use poem_openapi::Multipart;
use poem_openapi::payload::{Attachment, PlainText, Json};
use poem_openapi::types::multipart::Upload;
use poem_openapi::{ApiResponse, OpenApi, param::Path};
use serde::{Deserialize, Serialize};
use futures_util::StreamExt;

pub struct AssetsApi;

fn is_valid_asset_type(filename: &str) -> bool {
    let filename_lower = filename.to_lowercase();
    
    // Image file extensions
    let image_extensions = [
        ".jpg", ".jpeg", ".png", ".gif", ".bmp", ".webp", ".svg", ".tiff", ".tif", ".ico"
    ];
    
    // Audio file extensions  
    let audio_extensions = [
        ".mp3", ".wav", ".flac", ".aac", ".ogg", ".m4a", ".wma", ".opus"
    ];
    
    // Video file extensions
    let video_extensions = [
        ".mp4", ".avi", ".mov", ".wmv", ".flv", ".webm", ".mkv", ".m4v", ".3gp", ".ogv"
    ];
    
    image_extensions.iter().any(|ext| filename_lower.ends_with(ext)) ||
    audio_extensions.iter().any(|ext| filename_lower.ends_with(ext)) ||
    video_extensions.iter().any(|ext| filename_lower.ends_with(ext))
}

#[derive(Serialize, Deserialize, poem_openapi::Object)]
pub struct AssetInfo {
    pub name: String,
    pub size: u64,
    pub last_modified: String,
}

#[derive(Serialize, Deserialize, poem_openapi::Object)]
pub struct ListAssetsResponse {
    pub assets: Vec<String>,
    pub total_count: usize,
}

#[derive(Serialize, Deserialize, poem_openapi::Object)]
pub struct BatchAssetInfoRequest {
    pub asset_names: Vec<String>,
}

#[derive(Serialize, Deserialize, poem_openapi::Object)]
pub struct BatchAssetInfoResponse {
    pub assets: Vec<AssetInfo>,
}

#[derive(ApiResponse)]
enum GetImageResponse {
    #[oai(status = 200)]
    Ok(Attachment<Vec<u8>>),
    #[oai(status = 404)]
    NotFound,
}

#[derive(ApiResponse)]
enum ListAssetsApiResponse {
    #[oai(status = 200)]
    Ok(Json<ListAssetsResponse>),
}

#[derive(ApiResponse)]
enum AssetInfoResponse {
    #[oai(status = 200)]
    Ok(Json<AssetInfo>),
    #[oai(status = 404)]
    NotFound,
}

#[derive(ApiResponse)]
enum BatchAssetInfoApiResponse {
    #[oai(status = 200)]
    Ok(Json<BatchAssetInfoResponse>),
}

#[derive(ApiResponse)]
enum PutAssetResponse {
    #[oai(status = 200)]
    Ok(PlainText<String>),
    #[oai(status = 415)]
    UnsupportedMediaType,
}

#[derive(Multipart, Debug)]
pub struct PutImageRequest {
    pub asset: Upload,
}

#[OpenApi(prefix_path = "/assets", tag = "ApiTags::Assets")]
impl AssetsApi {
    #[oai(method = "get", path = "/:asset")]
    async fn get_asset(
        &self,
        asset: Path<String>,
        object_storage: Data<&ObjectStorage>,
    ) -> Result<GetImageResponse> {
        let get_object_request = object_storage.get_object(ASSETS_FILE_BUCKET, &*asset);

        let response = match get_object_request.send().await {
            Ok(response) => response,
            Err(why) => match why {
                minio::s3::error::Error::HttpError(error) => {
                    if let Some(status) = error.status() {
                        if status.as_u16() == 404 {
                            return Ok(GetImageResponse::NotFound);
                        } else {
                            return Err(InternalServerError(error));
                        }
                    } else {
                        return Err(InternalServerError(error));
                    }
                }
                _ => return Err(InternalServerError(why)),
            },
        };

        let segmented_bytes = response
            .content
            .to_segmented_bytes()
            .await
            .map_err(InternalServerError)?;

        let bytes = segmented_bytes.to_bytes();
        let bytes = bytes.to_vec();

        let attachment = Attachment::new(bytes).filename(&*asset);

        return Ok(GetImageResponse::Ok(attachment));
    }
    #[oai(method = "put", path = "/")]
    async fn put_asset(
        &self,
        claims: BearerAuthorization,
        object_storage: Data<&ObjectStorage>,
        request: PutImageRequest,
    ) -> Result<PutAssetResponse> {
        if !claims.permissions.contains(&"add asset".to_string()) {
            return Err(Error::from_status(StatusCode::FORBIDDEN));
        }

        let asset = request.asset;

        let Some(name) = asset.file_name() else {
            return Err(Error::from_status(StatusCode::BAD_REQUEST));
        };
        let name = name.to_string();

        // Validate file type - only allow images, audio, and video files
        if !is_valid_asset_type(&name) {
            return Ok(PutAssetResponse::UnsupportedMediaType);
        }

        let contents = asset.into_vec().await.unwrap();

        let put_object_request = object_storage.put_object(
            ASSETS_FILE_BUCKET,
            &*name,
            SegmentedBytes::from(Bytes::from(contents)),
        );

        put_object_request
            .send()
            .await
            .unwrap();

        Ok(PutAssetResponse::Ok(PlainText(format!("/assets/{}", name))))
    }

    #[oai(method = "get", path = "/")]
    async fn list_assets(
        &self,
        object_storage: Data<&ObjectStorage>,
    ) -> Result<ListAssetsApiResponse> {
        let mut stream = (**object_storage)
            .list_objects(ASSETS_FILE_BUCKET)
            .recursive(true)
            .use_api_v1(false) // use v2
            .to_stream()
            .await;
        
        let mut asset_names = Vec::new();
        
        while let Some(result) = stream.next().await {
            match result {
                Ok(response) => {
                    for object in response.contents {
                        asset_names.push(object.name);
                    }
                }
                Err(e) => return Err(InternalServerError(e)),
            }
        }
        
        let total_count = asset_names.len();
        
        Ok(ListAssetsApiResponse::Ok(Json(ListAssetsResponse {
            assets: asset_names,
            total_count,
        })))
    }

    #[oai(method = "get", path = "/:asset/info")]
    async fn get_asset_info(
        &self,
        asset: Path<String>,
        object_storage: Data<&ObjectStorage>,
    ) -> Result<AssetInfoResponse> {
        let stat_request = object_storage.stat_object(ASSETS_FILE_BUCKET, &*asset);

        let response = match stat_request.send().await {
            Ok(response) => response,
            Err(why) => match why {
                minio::s3::error::Error::HttpError(error) => {
                    if let Some(status) = error.status() {
                        if status.as_u16() == 404 {
                            return Ok(AssetInfoResponse::NotFound);
                        } else {
                            return Err(InternalServerError(error));
                        }
                    } else {
                        return Err(InternalServerError(error));
                    }
                }
                _ => return Err(InternalServerError(why)),
            },
        };

        let asset_info = AssetInfo {
            name: response.object,
            size: response.size as u64,
            last_modified: response.last_modified.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
        };

        Ok(AssetInfoResponse::Ok(Json(asset_info)))
    }

    #[oai(method = "post", path = "/batch/info")]
    async fn get_batch_asset_info(
        &self,
        object_storage: Data<&ObjectStorage>,
        request: Json<BatchAssetInfoRequest>,
    ) -> Result<BatchAssetInfoApiResponse> {
        let mut assets = Vec::new();

        for asset_name in &request.asset_names {
            let stat_request = object_storage.stat_object(ASSETS_FILE_BUCKET, asset_name);

            match stat_request.send().await {
                Ok(response) => {
                    assets.push(AssetInfo {
                        name: response.object,
                        size: response.size as u64,
                        last_modified: response.last_modified.map(|dt| dt.to_rfc3339()).unwrap_or_default(),
                    });
                }
                Err(_) => {
                    // Skip assets that don't exist or can't be accessed
                    continue;
                }
            }
        }

        Ok(BatchAssetInfoApiResponse::Ok(Json(BatchAssetInfoResponse {
            assets,
        })))
    }
}
