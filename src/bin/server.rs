//! CiteRight Web Service
//! Run with: cargo run --features server --bin citeright-server
//! Or set CITERIGHT_PORT to change the port (default: 3000)

use axum::{
    extract::Multipart,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::Serialize;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::info;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("citeright_server=info".parse().unwrap())
                .add_directive("citeright=info".parse().unwrap()),
        )
        .compact()
        .init();

    let app = Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/v1/verify", post(handle_verify))
        .layer(CorsLayer::permissive());

    let port: u16 = std::env::var("CITERIGHT_PORT")
        .unwrap_or_else(|_| "3000".to_string())
        .parse()
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("CiteRight server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../../templates/index.html"))
}

async fn handle_verify(mut multipart: Multipart) -> impl IntoResponse {
    // Extract file from multipart
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut filename = "document".to_string();
    let mut live = false;
    let mut token: Option<String> = None;
    let mut attorney_name: Option<String> = None;
    let mut bar_number: Option<String> = None;
    let mut jurisdiction: Option<String> = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name().map(|s| s.to_string()).as_deref() {
            Some("file") => {
                filename = field.file_name()
                    .unwrap_or("document")
                    .to_string();
                match field.bytes().await.map(|b| b.to_vec()) {
                    Ok(bytes) => file_bytes = Some(bytes),
                    Err(e) => return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({"error": format!("Failed to read file: {}", e)}))
                    ).into_response(),
                }
            }
            Some("live") => {
                if let Ok(val) = field.text().await {
                    live = val.trim() == "true" || val.trim() == "1";
                }
            }
            Some("token") => {
                if let Ok(val) = field.text().await {
                    let val: String = val;
                    if !val.is_empty() { token = Some(val); }
                }
            }
            Some("attorney_name") => {
                if let Ok(val) = field.text().await {
                    let val: String = val;
                    if !val.is_empty() { attorney_name = Some(val); }
                }
            }
            Some("bar_number") => {
                if let Ok(val) = field.text().await {
                    let val: String = val;
                    if !val.is_empty() { bar_number = Some(val); }
                }
            }
            Some("jurisdiction") => {
                if let Ok(val) = field.text().await {
                    let val: String = val;
                    if !val.is_empty() { jurisdiction = Some(val); }
                }
            }
            _ => {}
        }
    }

    let bytes = match file_bytes {
        Some(b) => b,
        None => return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "No file uploaded"}))
        ).into_response(),
    };

    info!("Received file: {} ({} bytes), live={}", filename, bytes.len(), live);

    // Write to temp file for pipeline processing
    let tmp_dir = std::env::temp_dir();
    let ext = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("md");
    let tmp_path = tmp_dir.join(format!("citeright_{}.{}", uuid::Uuid::new_v4(), ext));
    let out_dir = tmp_dir.join(format!("citeright_out_{}", uuid::Uuid::new_v4()));

    if let Err(e) = std::fs::write(&tmp_path, &bytes) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to write temp file: {}", e)}))
        ).into_response();
    }

    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to create output dir: {}", e)}))
        ).into_response();
    }

    // Resolve token from field or env
    let resolved_token = token
        .or_else(|| std::env::var("COURTLISTENER_TOKEN").ok());

    // Run pipeline
    let result = tokio::task::spawn_blocking({
        let tmp_path = tmp_path.clone();
        let out_dir = out_dir.clone();
        let resolved_token = resolved_token.clone();
        let attorney_name = attorney_name.clone();
        let bar_number = bar_number.clone();
        let jurisdiction = jurisdiction.clone();
        move || run_pipeline(tmp_path, out_dir, live, resolved_token, attorney_name, bar_number, jurisdiction)
    }).await;

    // Cleanup temp input file
    let _ = std::fs::remove_file(&tmp_path);

    match result {
        Ok(Ok(response)) => {
            // Cleanup output dir
            let _ = std::fs::remove_dir_all(&out_dir);
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(Err(e)) => {
            let _ = std::fs::remove_dir_all(&out_dir);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()}))
            ).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Pipeline task failed: {}", e)}))
        ).into_response(),
    }
}

fn run_pipeline(
    input_path: std::path::PathBuf,
    out_dir: std::path::PathBuf,
    live: bool,
    token: Option<String>,
    attorney_name: Option<String>,
    bar_number: Option<String>,
    jurisdiction: Option<String>,
) -> anyhow::Result<serde_json::Value> {
    use citeright::{document, claims, planner, candidates, artifact, audit};
    use citeright::models::VerificationStatus;

    let text = document::read_document(&input_path)?;
    let legal_claims = claims::extract_legal_claims(&text)?;
    let needs = planner::plan_citation_needs(&legal_claims)?;
    let cands = candidates::generate_candidates(&needs)?;

    // Use live or fixture lookup
    let lookups = if live {
        use citeright::courtlistener::CitationLookup;
        let client = citeright::courtlistener::CourtListenerClient::new(token.clone(), None)?;
        client.lookup_text(&text)?
    } else {
        // Use fixture from env var or default path
        let fixture_path = std::env::var("CITERIGHT_FIXTURE")
            .unwrap_or_else(|_| "fixtures/courtlistener_fixture.json".to_string());
        let fixture_path = std::path::Path::new(&fixture_path);
        if fixture_path.exists() {
            use citeright::courtlistener::CitationLookup;
            citeright::courtlistener::FixtureLookup::from_file(fixture_path)?
                .lookup_text(&text)?
        } else {
            vec![]
        }
    };

    let artifacts = artifact::artifacts_from_lookup_results(&lookups)?;
    let verified_count = artifacts.iter().filter(|a| a.verification_status == VerificationStatus::Verified).count();
    let unverified_count = artifacts.len() - verified_count;

    // Use empty resolutions/selections for server mode (no fixture path)
    let resolutions: Vec<citeright::resolver::CandidateResolution> = vec![];
    let selections: Vec<citeright::selector::SelectedCitation> = vec![];

    let receipt = audit::write_ai_audit(
        &out_dir,
        &input_path,
        &legal_claims,
        &needs,
        &cands,
        &resolutions,
        &selections,
        audit::AttestationParams {
            attorney_name,
            bar_number,
            jurisdiction,
            verified_count,
            unverified_count,
        },
    )?;

    Ok(serde_json::json!({
        "audit_id": receipt.audit_id,
        "input_digest": receipt.input_digest,
        "verified_count": verified_count,
        "unverified_count": unverified_count,
        "artifacts": serde_json::to_value(&artifacts)?,
        "attestation_text": receipt.attestation.attestation_text,
        "receipt_digest": receipt.receipt_digest,
    }))
}
