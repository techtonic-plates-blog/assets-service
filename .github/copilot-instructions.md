# Copilot Instructions for Assets Service

## Architecture Overview

This is a Rust microservice for the "Techtonic Plates Blog" that provides file asset management through a REST API. The service acts as a bridge between web clients and MinIO object storage, with JWT-based authentication.

**Key architectural components:**
- **Poem framework**: Web server with OpenAPI integration (not Axum/Actix) - [docs](https://docs.rs/poem/latest/poem/)
- **Poem OpenAPI**: Auto-generated API docs and validation - [docs](https://docs.rs/poem-openapi/latest/poem_openapi/)
- **MinIO client**: Object storage abstraction in `connections/object_storage.rs` - [docs](https://docs.rs/minio/latest/minio/)
- **JWT Auth**: RSA public key validation in `auth/mod.rs`
- **Modular routing**: API endpoints in `routes/` with OpenAPI annotations

## Critical Development Patterns

### Environment Configuration
Configuration uses `once_cell::Lazy` singleton pattern in `config.rs`. All config is environment-driven:
```rust
pub static CONFIG: Lazy<AppConfig> = Lazy::new(|| AppConfig {
    minio_url: env::var("MINIO_URL").expect("Could not get minio url"),
    // ... other required env vars
});
```

### Object Storage Abstraction
The `ObjectStorage` wrapper in `connections/object_storage.rs` uses `Deref`/`DerefMut` to expose MinIO client methods directly:
```rust
impl Deref for ObjectStorage {
    type Target = MinioClient;
    fn deref(&self) -> &Self::Target { &self.0 }
}
```

### API Response Patterns
All endpoints use custom `ApiResponse` enums for type-safe HTTP responses:
```rust
#[derive(ApiResponse)]
enum GetImageResponse {
    #[oai(status = 200)]
    Ok(Attachment<Vec<u8>>),
    #[oai(status = 404)]
    NotFound,
}
```

### Error Handling Convention
Use `poem::error::InternalServerError` wrapper for external errors, with explicit 404 detection for MinIO HTTP errors.

## Development Workflow

### Local Development
```bash
# Development with hot reload (requires cargo-watch)
cargo watch -x run

# Container development
docker compose -f compose.dev.yaml up
```

### Required Environment Variables
Set in `.dev.env` for development:
- `MINIO_URL`, `MINIO_ACCESS`, `MINIO_SECRET`: Object storage credentials
- `JWT_PUBLIC_KEY`: RSA public key for token validation (with `\n` escaping)

### API Documentation
- Swagger UI: `/docs/swagger`
- Scalar docs: `/docs/`
- OpenAPI spec: `/docs/api.json`, `/docs/api.yaml`

## Project-Specific Conventions

### Module Organization
- `setup/mod.rs`: Dependency injection setup returning `SetupResult`
- `routes/mod.rs`: Combines all API modules using tuple syntax
- One API struct per resource (e.g., `AssetsApi`)

### MinIO Integration
- Fixed bucket: `ASSETS_FILE_BUCKET = "assets-files"`
- Use `SegmentedBytes` for file uploads/downloads
- Stream-based listing with `futures_util::StreamExt`

### JWT Security Scheme
```rust
#[derive(SecurityScheme)]
#[oai(ty = "bearer", checker = "key_checker")]
pub struct BearerAuthorization(pub Claims);
```

## Key Files for Understanding

- `src/main.rs`: Entry point and Poem server setup
- `src/routes/assets.rs`: Core asset management endpoints
- `src/connections/object_storage.rs`: MinIO client wrapper
- `src/auth/mod.rs`: JWT validation security scheme
- `Cargo.toml`: Note the binary name is `images-service` despite repo name
