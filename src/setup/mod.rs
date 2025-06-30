use crate::config;
use crate::connections::ObjectStorage;


pub fn get_object_storage() -> anyhow::Result<ObjectStorage> {
    Ok(ObjectStorage::new(
        config::CONFIG.minio_url.clone(),
        config::CONFIG.minio_access.clone(),
        config::CONFIG.minio_secret.clone(),
    )?)
}


pub struct SetupResult {
    pub object_storage: ObjectStorage,
}

pub async fn setup_all() -> anyhow::Result<SetupResult> {
    let object_storage = get_object_storage()?;
    Ok(SetupResult { object_storage })
}
