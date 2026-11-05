//! Local Android host. Native assets, SQLite and category inference stay private.
mod security;

use pocket_category_provider::{CategoryChoice, CategorySuggestion, SuggestionStatus};
use receipt_core::{
    Category, ReceiptDraft, ReceiptError, ReceiptStore, SaveReceiptRequest, Transaction,
};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_receipt_capture::{ReceiptCaptureExt, ReceiptImage};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HostError {
    code: &'static str,
    message: &'static str,
}
impl HostError {
    fn storage() -> Self {
        Self {
            code: "storageUnavailable",
            message: "Local receipt storage is unavailable. Please try again.",
        }
    }
    fn invalid_image() -> Self {
        Self {
            code: "invalidImage",
            message: "This receipt image is unavailable. Please capture or choose it again.",
        }
    }
    fn invalid_input() -> Self {
        Self {
            code: "invalidInput",
            message: "Please check the receipt details.",
        }
    }
}
impl From<ReceiptError> for HostError {
    fn from(error: ReceiptError) -> Self {
        match error {
            ReceiptError::ConfirmationRequired => Self {
                code: "confirmationRequired",
                message: "Confirm the reviewed receipt before saving.",
            },
            ReceiptError::ConflictingSave => Self {
                code: "conflictingSave",
                message: "This receipt was already saved with different details.",
            },
            ReceiptError::DraftAlreadySaved => Self {
                code: "alreadySaved",
                message: "This receipt has already been saved.",
            },
            ReceiptError::DraftNotFound => Self {
                code: "draftNotFound",
                message: "This receipt draft is no longer available.",
            },
            ReceiptError::Validation(_) => Self::invalid_input(),
            ReceiptError::Storage => Self::storage(),
        }
    }
}
impl From<tauri_plugin_receipt_capture::Error> for HostError {
    fn from(error: tauri_plugin_receipt_capture::Error) -> Self {
        use tauri_plugin_receipt_capture::Error;
        match error {
            Error::Cancelled => Self {
                code: "cancelled",
                message: "Receipt selection was cancelled.",
            },
            Error::Busy => Self {
                code: "busy",
                message: "A receipt operation is already in progress.",
            },
            Error::ImageTooLarge => Self {
                code: "imageTooLarge",
                message: "This image is too large. Please choose a smaller image.",
            },
            Error::InvalidFile => Self::invalid_image(),
            _ => Self {
                code: "nativeUnavailable",
                message: "Receipt capture is available in the Android app.",
            },
        }
    }
}
type HostResult<T> = Result<T, HostError>;

#[derive(Clone)]
struct HostState {
    database_path: PathBuf,
    store: Arc<Mutex<Option<Arc<ReceiptStore>>>>,
    database_slots: Arc<tokio::sync::Semaphore>,
    asset_slots: Arc<tokio::sync::Semaphore>,
    receipt_lifecycle: Arc<tokio::sync::Mutex<()>>,
}
impl HostState {
    fn new(database_path: PathBuf) -> Self {
        Self {
            database_path,
            store: Arc::new(Mutex::new(None)),
            database_slots: Arc::new(tokio::sync::Semaphore::new(4)),
            asset_slots: Arc::new(tokio::sync::Semaphore::new(2)),
            receipt_lifecycle: Arc::new(tokio::sync::Mutex::new(())),
        }
    }
}
/// All DB opening, parsing and SQLite operations run on bounded blocking workers.
async fn with_store<T, F>(state: HostState, operation: F) -> HostResult<T>
where
    T: Send + 'static,
    F: FnOnce(&ReceiptStore) -> receipt_core::Result<T> + Send + 'static,
{
    let permit = state
        .database_slots
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| HostError::storage())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let store = {
            let mut slot = state.store.lock().map_err(|_| HostError::storage())?;
            if slot.is_none() {
                let parent = state
                    .database_path
                    .parent()
                    .ok_or_else(HostError::storage)?;
                std::fs::create_dir_all(parent).map_err(|_| HostError::storage())?;
                *slot = Some(Arc::new(
                    ReceiptStore::open(&state.database_path).map_err(HostError::from)?,
                ));
            }
            slot.as_ref().ok_or_else(HostError::storage)?.clone()
        };
        operation(&store).map_err(HostError::from)
    })
    .await
    .map_err(|_| HostError::storage())?
}
fn validate_id(id: &str) -> HostResult<()> {
    if security::valid_reference(id) {
        Ok(())
    } else {
        Err(HostError::invalid_input())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CapturedReceipt {
    image_ref: String,
    preview_ref: String,
    width: u32,
    height: u32,
    mime_type: String,
}
async fn verify_capture(app: &AppHandle, image: ReceiptImage) -> HostResult<CapturedReceipt> {
    if image.file_ref != image.preview_ref || !security::valid_reference(&image.file_ref) {
        return Err(HostError::invalid_image());
    }
    let root = app
        .receipt_capture()
        .storage_root()
        .await
        .map_err(HostError::from)?;
    let reference = image.file_ref.clone();
    tauri::async_runtime::spawn_blocking(move || {
        security::resolve_owned_file(&root, &reference, security::AssetKind::Original)
            .map_err(|_| HostError::invalid_image())?;
        security::resolve_owned_file(&root, &reference, security::AssetKind::Preview)
            .map_err(|_| HostError::invalid_image())?;
        Ok::<(), HostError>(())
    })
    .await
    .map_err(|_| HostError::invalid_image())??;
    Ok(CapturedReceipt {
        image_ref: image.file_ref,
        preview_ref: image.preview_ref,
        width: image.width,
        height: image.height,
        mime_type: image.mime_type,
    })
}
#[tauri::command]
async fn capture_receipt(app: AppHandle) -> HostResult<CapturedReceipt> {
    let image = app
        .receipt_capture()
        .capture()
        .await
        .map_err(HostError::from)?;
    verify_capture(&app, image).await
}
#[tauri::command]
async fn import_receipt(app: AppHandle) -> HostResult<CapturedReceipt> {
    let image = app
        .receipt_capture()
        .import_receipt()
        .await
        .map_err(HostError::from)?;
    verify_capture(&app, image).await
}
#[tauri::command]
async fn list_transactions(state: State<'_, HostState>) -> HostResult<Vec<Transaction>> {
    with_store(state.inner().clone(), |store| {
        store.list_recent_transactions(200)
    })
    .await
}
#[tauri::command]
async fn list_categories(state: State<'_, HostState>) -> HostResult<Vec<Category>> {
    with_store(state.inner().clone(), |store| store.list_categories()).await
}
#[tauri::command]
async fn draft_from_receipt(
    app: AppHandle,
    state: State<'_, HostState>,
    image_ref: String,
) -> HostResult<ReceiptDraft> {
    validate_id(&image_ref)?;
    let _lifecycle = state.receipt_lifecycle.lock().await;
    let root = app
        .receipt_capture()
        .storage_root()
        .await
        .map_err(HostError::from)?;
    let reference = image_ref.clone();
    let preview_available = tauri::async_runtime::spawn_blocking(move || {
        security::resolve_owned_file(&root, &reference, security::AssetKind::Original)
            .map_err(|_| HostError::invalid_image())?;
        Ok::<bool, HostError>(
            security::resolve_owned_file(&root, &reference, security::AssetKind::Preview).is_ok(),
        )
    })
    .await
    .map_err(|_| HostError::invalid_image())??;
    let recognized = tokio::time::timeout(
        Duration::from_secs(25),
        app.receipt_capture().recognize(&image_ref),
    )
    .await;
    let (ocr_text, ocr_failed) = match recognized {
        Ok(Ok(result)) if result.text.len() <= receipt_core::MAX_OCR_BYTES => (result.text, false),
        _ => {
            let _ =
                tokio::time::timeout(Duration::from_secs(1), app.receipt_capture().cancel()).await;
            (String::new(), true)
        }
    };
    let mut draft = with_store(state.inner().clone(), move |store| {
        let preview = preview_available.then_some(image_ref.as_str());
        store.create_draft_with_preview(&image_ref, preview, &ocr_text)
    })
    .await?;
    if ocr_failed {
        draft.warnings.push("Text recognition was unavailable. The image is retained; enter the receipt details manually.".into());
    }
    Ok(draft)
}
#[tauri::command]
async fn suggest_category(
    state: State<'_, HostState>,
    draft_id: String,
) -> HostResult<CategorySuggestion> {
    validate_id(&draft_id)?;
    let (draft, categories) = with_store(state.inner().clone(), move |store| {
        Ok((store.get_draft(&draft_id)?, store.list_categories()?))
    })
    .await?;
    let choices: Vec<CategoryChoice> = categories
        .into_iter()
        .map(|category| CategoryChoice {
            id: category.id,
            label: category.label,
        })
        .collect();
    let task = tauri::async_runtime::spawn(async move {
        let result = tokio::time::timeout(
            Duration::from_secs(3),
            pocket_category_provider::suggest_category(&draft.ocr_text, &choices),
        )
        .await;
        match result {
            Ok(suggestion)
                if suggestion.status == SuggestionStatus::Suggested
                    && suggestion
                        .category_id
                        .as_ref()
                        .is_some_and(|id| choices.iter().any(|choice| &choice.id == id)) =>
            {
                suggestion
            }
            Ok(suggestion) if suggestion.status == SuggestionStatus::Unavailable => {
                CategorySuggestion {
                    category_id: None,
                    status: SuggestionStatus::Unavailable,
                }
            }
            _ => CategorySuggestion {
                category_id: None,
                status: SuggestionStatus::Failed,
            },
        }
    });
    Ok(task.await.unwrap_or(CategorySuggestion {
        category_id: None,
        status: SuggestionStatus::Failed,
    }))
}
#[tauri::command]
async fn save_receipt(
    state: State<'_, HostState>,
    request: SaveReceiptRequest,
) -> HostResult<Transaction> {
    validate_id(&request.draft_id)?;
    with_store(state.inner().clone(), move |store| {
        store.save_receipt(&request)
    })
    .await
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscardResult {
    discarded: bool,
    image_cleanup_pending: bool,
}
#[tauri::command]
async fn discard_receipt(
    app: AppHandle,
    state: State<'_, HostState>,
    draft_id: String,
) -> HostResult<DiscardResult> {
    validate_id(&draft_id)?;
    let _lifecycle = state.receipt_lifecycle.lock().await;
    // One persisted draft per image and this lifecycle lock prevent a new draft
    // from racing DB deletion and native cleanup. SQL rejects saved drafts.
    let image_ref = with_store(state.inner().clone(), move |store| {
        let draft = store.get_draft(&draft_id)?;
        if !security::valid_reference(&draft.image_ref) {
            return Err(ReceiptError::Validation("Invalid image reference".into()));
        }
        store.discard_draft(&draft_id)?;
        Ok(draft.image_ref)
    })
    .await?;
    let image_cleanup_pending = app.receipt_capture().discard(&image_ref).await.is_err();
    Ok(DiscardResult {
        discarded: true,
        image_cleanup_pending,
    })
}

async fn protocol_preview(app: AppHandle, reference: String) -> HostResult<Vec<u8>> {
    if !security::valid_reference(&reference) {
        return Err(HostError::invalid_image());
    }
    let state = app.state::<HostState>().inner().clone();
    let permit = state
        .asset_slots
        .acquire_owned()
        .await
        .map_err(|_| HostError::invalid_image())?;
    let root = app
        .receipt_capture()
        .storage_root()
        .await
        .map_err(HostError::from)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        security::read_preview(&root, &reference).map_err(|_| HostError::invalid_image())
    })
    .await
    .map_err(|_| HostError::invalid_image())?
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_receipt_capture::init())
        .setup(|app| {
            let directory = app
                .path()
                .app_data_dir()
                .map_err(|_| std::io::Error::other("Local app storage path is unavailable"))?;
            app.manage(HostState::new(directory.join("receipts.sqlite3")));
            Ok(())
        })
        .register_asynchronous_uri_scheme_protocol("receipt", |context, request, responder| {
            let app = context.app_handle().clone();
            let allowed = context.webview_label() == "main"
                && request.method() == tauri::http::Method::GET
                && request.uri().query().is_none();
            let reference = request
                .uri()
                .path()
                .strip_prefix('/')
                .unwrap_or("")
                .to_owned();
            tauri::async_runtime::spawn(async move {
                let result = if allowed {
                    protocol_preview(app, reference).await
                } else {
                    Err(HostError::invalid_image())
                };
                let (status, bytes) = match result {
                    Ok(bytes) => (200, bytes),
                    Err(_) => (404, Vec::new()),
                };
                let response = tauri::http::Response::builder()
                    .status(status)
                    .header("Content-Type", "image/jpeg")
                    .header("Cache-Control", "no-store")
                    .header("X-Content-Type-Options", "nosniff")
                    .body(bytes)
                    .expect("constant receipt response headers");
                responder.respond(response);
            });
        })
        .invoke_handler(tauri::generate_handler![
            list_transactions,
            list_categories,
            capture_receipt,
            import_receipt,
            draft_from_receipt,
            suggest_category,
            save_receipt,
            discard_receipt
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|_| panic!("The local receipt application could not start"));
}
