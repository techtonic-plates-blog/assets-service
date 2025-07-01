use crate::connections::ObjectStorage;
use crate::connections::object_storage::IMAGES_FILE_BUCKET;
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

pub struct ImagesApi;
#[derive(ApiResponse)]
enum GetImageResponse {
    #[oai(status = 200)]
    Ok(Attachment<Vec<u8>>),
    #[oai(status = 404)]
    NotFound,
}

#[derive(Multipart, Debug)]
pub struct PutImageRequest {
    pub image: Upload,
}

#[OpenApi(prefix_path = "/images", tag = "ApiTags::Posts")]
impl ImagesApi {
    #[oai(method = "get", path = "/:image")]
    async fn get_image(
        &self,
        image: Path<String>,
        object_storage: Data<&ObjectStorage>,
    ) -> Result<GetImageResponse> {
        let get_object_request = object_storage.get_object(IMAGES_FILE_BUCKET, &*image);

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

        let attachment = Attachment::new(bytes).filename(&*image);

        return Ok(GetImageResponse::Ok(attachment));
    }
    #[oai(method = "put", path = "/")]
    async fn put_image(
        &self,
        object_storage: Data<&ObjectStorage>,
        request: PutImageRequest,
    ) -> Result<PlainText<String>> {
        let image = request.image;

        let Some(name) = image.file_name() else {
            return Err(Error::from_status(StatusCode::BAD_REQUEST));
        };
        let name = name.to_string();

        let contents = image.into_vec().await.map_err(InternalServerError)?;

        let put_object_request = object_storage.put_object(
            IMAGES_FILE_BUCKET,
            &*name,
            SegmentedBytes::from(Bytes::from(contents)),
        );

        put_object_request
            .send()
            .await
            .map_err(InternalServerError)?;

        Ok(PlainText(format!("/images/{}", name)))
    }
}
