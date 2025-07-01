
use poem_openapi::{OpenApi, Tags};

mod assets;

#[derive(Debug, Tags)]
#[allow(dead_code)]
pub enum ApiTags {
    Assets,

}

pub struct RootApi;

#[OpenApi]
impl RootApi {
      #[oai(method = "get", path = "/healthcheck")]
      async fn healthcheck(&self) {

      }
}

pub fn api() -> impl OpenApi {
    (RootApi, assets::AssetsApi)
}