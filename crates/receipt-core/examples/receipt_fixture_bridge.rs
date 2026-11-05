//! Test-only loopback bridge for synthetic browser integration. Never shipped
//! inside the Android app. Exercises the real Rust parser and SQLite store; it
//! does not perform camera capture or image-to-text recognition.
use receipt_core::{ReceiptStore, SaveReceiptRequest};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{collections::HashMap, fs, path::PathBuf};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use uuid::Uuid;

const STANDARD: &str = "ALPENMARKT\nMusterstraße 12, 1010 Wien\nDEMOBELEG SYNTHETISCH\n05.11.2026 14:32\nBrot 3,49\nMilch 2,50\nObst 6,50\nGESAMT EUR 12,49\nBAR 20,00\nRÜCKGELD 7,51";
const ORIGINS: &[&str] = &[
    "http://localhost:1420",
    "http://127.0.0.1:1420",
    "http://localhost:4173",
    "http://127.0.0.1:4173",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Invocation {
    command: String,
    #[serde(default)]
    args: Value,
}
struct Fixtures {
    store: Option<ReceiptStore>,
    images: HashMap<String, String>,
    db: PathBuf,
    base: String,
}
impl Fixtures {
    fn store(&self) -> &ReceiptStore {
        self.store.as_ref().expect("fixture store initialized")
    }
    fn reset(&mut self) -> Result<(), String> {
        drop(self.store.take());
        for path in [
            self.db.clone(),
            PathBuf::from(format!("{}-wal", self.db.display())),
            PathBuf::from(format!("{}-shm", self.db.display())),
        ] {
            if path.exists() {
                fs::remove_file(path).map_err(|_| "Fixture reset failed")?;
            }
        }
        self.store = Some(ReceiptStore::open(&self.db).map_err(|_| "Fixture storage unavailable")?);
        self.images.clear();
        Ok(())
    }
    fn invoke(&mut self, request: Invocation) -> Result<Value, String> {
        let field = |key: &str| {
            request
                .args
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| "Invalid fixture request".to_owned())
        };
        let value = match request.command.as_str() {
            "list_categories" => serde_json::to_value(
                self.store()
                    .list_categories()
                    .map_err(|_| "Fixture storage unavailable")?,
            ),
            "list_transactions" => serde_json::to_value(
                self.store()
                    .list_recent_transactions(100)
                    .map_err(|_| "Fixture storage unavailable")?,
            ),
            "capture_receipt" | "import_receipt" => {
                let scenario = request
                    .args
                    .get("scenario")
                    .and_then(Value::as_str)
                    .unwrap_or("standard");
                if !["standard", "ambiguous", "ocr_failure"].contains(&scenario) {
                    return Err("Unknown synthetic fixture".into());
                }
                let image_ref = Uuid::new_v4().to_string();
                self.images.insert(image_ref.clone(), scenario.to_owned());
                Ok(
                    json!({"imageRef":image_ref,"previewRef":format!("{}/fixtures/{scenario}.svg",self.base)}),
                )
            }
            "draft_from_receipt" => {
                let image_ref = field("imageRef")?;
                let scenario = self
                    .images
                    .get(&image_ref)
                    .ok_or("Unknown fixture image")?
                    .clone();
                let ocr = match scenario.as_str() {
                    "ambiguous" => STANDARD.replace(
                        "GESAMT EUR 12,49",
                        "GESAMT EUR 12,49\nGESAMTBETRAG EUR 14,49",
                    ),
                    "ocr_failure" => String::new(),
                    _ => STANDARD.to_owned(),
                };
                let draft = self
                    .store()
                    .create_draft_with_preview(&image_ref, Some(&image_ref), &ocr)
                    .map_err(|_| "Fixture draft failed")?;
                let mut value =
                    serde_json::to_value(draft).map_err(|_| "Fixture response failed")?;
                value["previewRef"] = json!(format!("{}/fixtures/{scenario}.svg", self.base));
                Ok(value)
            }
            "suggest_category" => {
                self.store()
                    .get_draft(&field("draftId")?)
                    .map_err(|_| "Fixture draft unavailable")?;
                Ok(json!({"categoryId":null,"status":"unavailable"}))
            }
            "save_receipt" => {
                let save: SaveReceiptRequest = serde_json::from_value(
                    request
                        .args
                        .get("request")
                        .cloned()
                        .ok_or("Missing save request")?,
                )
                .map_err(|_| "Invalid save request")?;
                let result = self
                    .store()
                    .save_receipt(&save)
                    .map_err(|error| error.to_string())?;
                serde_json::to_value(result)
            }
            "discard_receipt" => {
                self.store()
                    .discard_draft(&field("draftId")?)
                    .map_err(|_| "Fixture discard failed")?;
                Ok(json!({"discarded": true, "imageCleanupPending": false}))
            }
            _ => return Err("Unsupported fixture command".into()),
        };
        value.map_err(|_| "Fixture response failed".to_owned())
    }
}
fn respond(request: Request, status: u16, body: Vec<u8>, content_type: &str, origin: Option<&str>) {
    let mut response = Response::from_data(body).with_status_code(StatusCode(status));
    for (name, value) in [
        ("Content-Type", content_type),
        ("Cache-Control", "no-store"),
        ("X-Content-Type-Options", "nosniff"),
    ] {
        response.add_header(Header::from_bytes(name, value).expect("fixed HTTP header"));
    }
    if let Some(origin) = origin {
        response.add_header(
            Header::from_bytes("Access-Control-Allow-Origin", origin).expect("validated origin"),
        );
        response.add_header(
            Header::from_bytes("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
                .expect("fixed header"),
        );
        response.add_header(
            Header::from_bytes("Access-Control-Allow-Headers", "Content-Type")
                .expect("fixed header"),
        );
    }
    let _ = request.respond(response);
}
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port: u16 = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "8787".into())
        .parse()?;
    if port < 1024 {
        return Err("Use an unprivileged localhost fixture port".into());
    }
    let db = PathBuf::from("artifacts/browser-fixtures/receipts.sqlite");
    fs::create_dir_all(db.parent().expect("fixed parent"))?;
    let server = Server::http(("127.0.0.1", port))?;
    let mut fixtures = Fixtures {
        store: Some(ReceiptStore::open(&db)?),
        images: HashMap::new(),
        db,
        base: format!("http://127.0.0.1:{port}"),
    };
    eprintln!("Synthetic-only Rust receipt fixture bridge listening on 127.0.0.1:{port}");
    for mut request in server.incoming_requests() {
        let origin = request
            .headers()
            .iter()
            .find(|header| header.field.equiv("Origin"))
            .map(|header| header.value.as_str().to_owned());
        if origin
            .as_deref()
            .is_some_and(|value| !ORIGINS.contains(&value))
        {
            respond(request, 403, b"{}".to_vec(), "application/json", None);
            continue;
        }
        if request.method() == &Method::Options {
            respond(request, 204, vec![], "application/json", origin.as_deref());
            continue;
        }
        if request.method() == &Method::Get && request.url() == "/health" {
            respond(
                request,
                200,
                b"{\"synthetic\":true}".to_vec(),
                "application/json",
                origin.as_deref(),
            );
            continue;
        }
        if request.method() == &Method::Get
            && matches!(
                request.url(),
                "/fixtures/standard.svg" | "/fixtures/ambiguous.svg" | "/fixtures/ocr_failure.svg"
            )
        {
            let path = if request.url() == "/fixtures/ambiguous.svg" {
                "static/demo-receipt-ambiguous.svg"
            } else {
                "static/demo-receipt.svg"
            };
            let (status, body) = match fs::read(path) {
                Ok(bytes) => (200, bytes),
                Err(_) => (404, vec![]),
            };
            respond(request, status, body, "image/svg+xml", origin.as_deref());
            continue;
        }
        if request.method() == &Method::Post && request.url() == "/__test/reset" {
            let status = if fixtures.reset().is_ok() { 200 } else { 500 };
            respond(
                request,
                status,
                b"{}".to_vec(),
                "application/json",
                origin.as_deref(),
            );
            continue;
        }
        if request.method() != &Method::Post || request.url() != "/invoke" {
            respond(
                request,
                404,
                b"{}".to_vec(),
                "application/json",
                origin.as_deref(),
            );
            continue;
        }
        let mut body = String::new();
        use std::io::Read;
        let read = request.as_reader().take(65_537).read_to_string(&mut body);
        let result = if read.is_err() || body.len() > 65_536 {
            Err("Invalid fixture request".to_owned())
        } else {
            serde_json::from_str::<Invocation>(&body)
                .map_err(|_| "Invalid fixture request".to_owned())
                .and_then(|invocation| fixtures.invoke(invocation))
        };
        let (status, value) = match result {
            Ok(value) => (200, value),
            Err(message) => (400, json!({"message":message})),
        };
        respond(
            request,
            status,
            serde_json::to_vec(&value)?,
            "application/json",
            origin.as_deref(),
        );
    }
    Ok(())
}
