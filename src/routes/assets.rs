use crate::connections::ObjectStorage;
use crate::connections::object_storage::ASSETS_FILE_BUCKET;
use crate::routes::ApiTags;
use bytes::Bytes;
use minio::s3::segmented_bytes::SegmentedBytes;
use minio::s3::types::S3Api;
use poem::Error;
use poem::http::StatusCode;
use poem::{Result, error::InternalServerError, web::Data};
use poem_openapi::Multipart;
use poem_openapi::payload::{Attachment, PlainText};
use poem_openapi::types::multipart::Upload;
use poem_openapi::{ApiResponse, OpenApi, param::Path};

pub struct AssetsApi;
#[derive(ApiResponse)]
enum GetImageResponse {
    #[oai(status = 200)]
    Ok(Attachment<Vec<u8>>),
    #[oai(status = 404)]
    NotFound,
}

#[derive(Multipart, Debug)]
pub struct PutImageRequest {
    pub asset: Upload,
}

#[OpenApi(prefix_path = "/assets", tag = "ApiTags::Posts")]
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
        object_storage: Data<&ObjectStorage>,
        request: PutImageRequest,
    ) -> Result<PlainText<String>> {
        let asset = request.asset;

        let Some(name) = asset.file_name() else {
            return Err(Error::from_status(StatusCode::BAD_REQUEST));
        };
        let name = name.to_string();

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

        Ok(PlainText(format!("/assets/{}", name)))
    }
}
