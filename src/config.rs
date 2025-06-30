use once_cell::sync::Lazy;
use std::env;

pub struct AppConfig {
    pub minio_url: String,
    pub minio_access: String,
    pub minio_secret: String,
}

pub static CONFIG: Lazy<AppConfig> = Lazy::new(|| AppConfig {
    minio_url: env::var("MINIO_URL").expect("Could not get minio url"),
    minio_access: env::var("MINIO_ACCESS").expect("Could not get minio access key"),
    minio_secret: env::var("MINIO_SECRET").expect("Could not get minio secret key"),
});
