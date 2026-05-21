//! OpenAPI documentation with `utoipa` and Swagger UI.

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

use crate::http_schemas::{
    ErrorBody, ExtractionHealthHttp, HealthReportHttp, IndexRebuildHttp, ServerInfoHttp,
};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rust PDF MCP API",
        version = "0.3.0",
        description = "PDF extraction, knowledge compilation, and full-text search HTTP API",
    ),
    servers(
        (url = "http://localhost:8000", description = "Local development"),
    ),
    tags(
        (name = "wiki", description = "Wiki browser read API"),
        (name = "management", description = "Health, compile, and index management"),
        (name = "collaboration", description = "Share links"),
    ),
    modifiers(&SecurityAddon),
    paths(
        crate::api_doc::health_path,
        crate::api_doc::server_info_path,
        crate::api_doc::compile_status_path,
        crate::api_doc::index_rebuild_path,
        crate::api_doc::index_status_path,
    ),
    components(schemas(
        HealthReportHttp,
        ExtractionHealthHttp,
        IndexRebuildHttp,
        ServerInfoHttp,
        ErrorBody,
    )),
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new().scheme(HttpAuthScheme::Bearer).bearer_format("JWT").build(),
                ),
            );
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/health",
    tag = "management",
    params(("kb_id" = Option<String>, Query, description = "Knowledge base id")),
    responses(
        (status = 200, description = "Health report", body = HealthReportHttp),
        (status = 500, description = "Error", body = ErrorBody),
    )
)]
#[allow(dead_code)]
fn health_path() {}

#[utoipa::path(
    get,
    path = "/api/server-info",
    tag = "management",
    responses(
        (status = 200, description = "MCP mode and config snippet", body = ServerInfoHttp),
    )
)]
#[allow(dead_code)]
fn server_info_path() {}

#[utoipa::path(
    get,
    path = "/api/compile/status",
    tag = "management",
    params(("kb_id" = Option<String>, Query, description = "Knowledge base id")),
    responses((status = 200, description = "Compile job view JSON object")),
)]
#[allow(dead_code)]
fn compile_status_path() {}

#[utoipa::path(
    post,
    path = "/api/index/rebuild",
    tag = "management",
    params(("kb_id" = Option<String>, Query, description = "Knowledge base id")),
    responses(
        (status = 200, description = "Rebuild stats", body = IndexRebuildHttp),
        (status = 500, description = "Error", body = ErrorBody),
    ),
)]
#[allow(dead_code)]
fn index_rebuild_path() {}

#[utoipa::path(
    get,
    path = "/api/index/status",
    tag = "management",
    params(("kb_id" = Option<String>, Query, description = "Knowledge base id")),
    responses((status = 200, description = "Last index rebuild metadata")),
)]
#[allow(dead_code)]
fn index_status_path() {}

/// Build a Router with Swagger UI at `/swagger-ui`.
pub fn openapi_router() -> axum::Router {
    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()).into()
}

/// Get the raw OpenAPI JSON spec.
pub fn openapi_json() -> String {
    ApiDoc::openapi().to_pretty_json().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::ApiDoc;
    use utoipa::OpenApi;

    #[test]
    fn openapi_has_registered_paths() {
        let doc = ApiDoc::openapi();
        assert!(!doc.paths.paths.is_empty(), "OpenAPI paths must not be empty");
    }

    fn fixture_path() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/openapi.json")
    }

    #[test]
    fn openapi_fixture_matches_code() {
        let expected = super::openapi_json();
        let path = fixture_path();
        assert!(
            path.exists(),
            "missing tests/fixtures/openapi.json — run: cargo test -p pdf-mcp api_doc::tests::write_openapi_fixture -- --ignored"
        );
        let on_disk = std::fs::read_to_string(&path).expect("read openapi fixture");
        assert_eq!(
            on_disk.trim(),
            expected.trim(),
            "OpenAPI fixture out of date — run: cargo test -p pdf-mcp api_doc::tests::write_openapi_fixture -- --ignored && npm run generate:api"
        );
    }

    #[test]
    #[ignore = "run manually to refresh tests/fixtures/openapi.json"]
    fn write_openapi_fixture() {
        let json = super::openapi_json();
        let path = fixture_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create fixtures dir");
        }
        std::fs::write(path, json).expect("write openapi.json");
    }
}
