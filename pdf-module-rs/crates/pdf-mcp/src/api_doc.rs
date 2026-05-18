//! OpenAPI documentation with `utoipa` and Swagger UI.
//!
//! Auto-generates OpenAPI 3.1 spec from Rust type annotations and
//! serves Swagger UI at `/swagger-ui` for interactive API exploration.
//!
//! # Usage
//!
//! 1. Annotate request/response types with `#[derive(ToSchema)]`
//! 2. Annotate handlers with `#[utoipa::path(...)]`
//! 3. Call `openapi_router()` to get a Router with `/swagger-ui` mounted
//!
//! ```ignore
//! let app = Router::new()
//!     .merge(api_doc::openapi_router())
//!     .merge(api_routes());
//! ```

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Rust PDF MCP API",
        version = "0.3.0",
        description = "PDF extraction, knowledge compilation, and full-text search API",
        contact(name = "rsut-pdf-mcp", url = "https://github.com/loonghao/rsut_pdf_mcp"),
    ),
    servers(
        (url = "http://localhost:8000", description = "Local development"),
        (url = "/api", description = "Production"),
    ),
    tags(
        (name = "extraction", description = "PDF text and structure extraction"),
        (name = "knowledge", description = "Knowledge base compilation and search"),
        (name = "management", description = "Server configuration and health"),
    ),
    modifiers(&SecurityAddon),
    paths(
        // Register api handlers here as they get #[utoipa::path] annotations
        // crate::http::api_health,
        // crate::http::api_wiki_tree,
        // crate::http::api_wiki_search,
    ),
    components(schemas(
        // Register DTO types here as they get #[derive(ToSchema)]
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
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("Enter JWT token: Bearer <token>"))
                        .build(),
                ),
            );
        }
    }
}

/// Build a Router with Swagger UI at `/swagger-ui`.
pub fn openapi_router() -> axum::Router {
    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()).into()
}

/// Get the raw OpenAPI JSON spec.
pub fn openapi_json() -> String {
    ApiDoc::openapi().to_pretty_json().unwrap_or_default()
}
