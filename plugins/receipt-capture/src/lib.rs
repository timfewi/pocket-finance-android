//! Android receipt I/O and bundled OCR. Host Rust is the only public caller;
//! no arbitrary path, content URI, shell, or network command is exposed to JS.
use serde::{Deserialize, Serialize};
use std::{marker::PhantomData, path::PathBuf};
#[cfg(target_os = "android")]
use tauri::plugin::{mobile::PluginInvokeError, PluginHandle};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReceiptImage {
    pub file_ref: String,
    /// Same opaque asset ID, resolved only as the preview rendition by the host.
    pub preview_ref: String,
    pub width: u32,
    pub height: u32,
    pub mime_type: String,
    pub preview_width: u32,
    pub preview_height: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OcrResult {
    pub text: String,
}

#[derive(Clone, Copy, Debug, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
pub enum Error {
    #[error("Receipt operation cancelled")]
    Cancelled,
    #[error("Receipt operation already in progress")]
    Busy,
    #[error("Receipt image is unavailable or invalid")]
    InvalidFile,
    #[error("Receipt image exceeds supported limits")]
    ImageTooLarge,
    #[error("Receipt recognition failed; enter the receipt manually")]
    OcrFailed,
    #[error("Receipt recognition timed out; enter the receipt manually")]
    Timeout,
    #[error("Native receipt capture is unavailable; enter the receipt manually")]
    Unavailable,
}

pub type Result<T> = std::result::Result<T, Error>;

/// Canonical lowercase UUID representation only; no filename or path components.
pub fn valid_file_ref(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            }
        })
}

pub struct ReceiptCapture<R: Runtime> {
    #[cfg(target_os = "android")]
    handle: PluginHandle<R>,
    _runtime: PhantomData<fn() -> R>,
}

impl<R: Runtime> ReceiptCapture<R> {
    pub async fn capture(&self) -> Result<ReceiptImage> {
        self.call("capture", serde_json::json!({})).await
    }

    pub async fn import_receipt(&self) -> Result<ReceiptImage> {
        self.call("importReceipt", serde_json::json!({})).await
    }

    pub async fn recognize(&self, file_ref: &str) -> Result<OcrResult> {
        if !valid_file_ref(file_ref) {
            return Err(Error::InvalidFile);
        }
        self.call("recognize", serde_json::json!({"fileRef":file_ref}))
            .await
    }

    /// Trusted host use only. Never return the result through a frontend command.
    pub async fn storage_root(&self) -> Result<PathBuf> {
        #[derive(Deserialize)]
        struct Storage {
            root: String,
        }
        let storage: Storage = self.call("storageRoot", serde_json::json!({})).await?;
        let path = PathBuf::from(storage.root);
        if !path.is_absolute() {
            return Err(Error::Unavailable);
        }
        Ok(path)
    }

    pub async fn discard(&self, file_ref: &str) -> Result<()> {
        if !valid_file_ref(file_ref) {
            return Err(Error::InvalidFile);
        }
        self.call("discard", serde_json::json!({"fileRef":file_ref}))
            .await
    }

    /// Stops delivery of the current result. Android's external camera/picker must
    /// still be dismissed by the user; ML Kit kernel work may finish in background.
    pub async fn cancel(&self) -> Result<()> {
        self.call("cancel", serde_json::json!({})).await
    }

    #[cfg(target_os = "android")]
    async fn call<T: serde::de::DeserializeOwned>(
        &self,
        command: &'static str,
        payload: serde_json::Value,
    ) -> Result<T> {
        self.handle
            .run_mobile_plugin_async(command, payload)
            .await
            .map_err(map_native_error)
    }

    #[cfg(not(target_os = "android"))]
    async fn call<T: serde::de::DeserializeOwned>(
        &self,
        _command: &'static str,
        _payload: serde_json::Value,
    ) -> Result<T> {
        Err(Error::Unavailable)
    }
}

#[cfg(target_os = "android")]
fn map_native_error(error: PluginInvokeError) -> Error {
    if let PluginInvokeError::InvokeRejected(response) = error {
        return match response.code.as_deref() {
            Some("cancelled") => Error::Cancelled,
            Some("busy") => Error::Busy,
            Some("invalidFile") => Error::InvalidFile,
            Some("imageTooLarge") => Error::ImageTooLarge,
            Some("ocrFailed") => Error::OcrFailed,
            Some("timeout") => Error::Timeout,
            _ => Error::Unavailable,
        };
    }
    Error::Unavailable
}

pub trait ReceiptCaptureExt<R: Runtime> {
    fn receipt_capture(&self) -> &ReceiptCapture<R>;
}
impl<R: Runtime, T: Manager<R>> ReceiptCaptureExt<R> for T {
    fn receipt_capture(&self) -> &ReceiptCapture<R> {
        self.state::<ReceiptCapture<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("receipt-capture")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let handle = api.register_android_plugin(
                "dev.pocketfinance.receiptcapture",
                "ReceiptCapturePlugin",
            )?;
            #[cfg(not(target_os = "android"))]
            let _ = api;
            app.manage(ReceiptCapture::<R> {
                #[cfg(target_os = "android")]
                handle,
                _runtime: PhantomData,
            });
            Ok(())
        })
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn references_cannot_encode_paths_or_content_uris() {
        assert!(valid_file_ref("01234567-89ab-4cde-8fab-0123456789ab"));
        for value in [
            "",
            "../receipt.jpg",
            "/data/user/0/private",
            "content://receipt/1",
            "01234567-89AB-4cde-8fab-0123456789ab",
            "0123456789ab4cde8fab0123456789ab",
            "01234567-89ab-4cde-8fab-0123456789ab.jpg",
        ] {
            assert!(!valid_file_ref(value));
        }
    }
}
