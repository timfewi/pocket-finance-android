//! Strict resolution of app-owned receipt assets. No webview path is accepted.
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub const MAX_ORIGINAL_BYTES: u64 = 32 * 1024 * 1024;
pub const MAX_PREVIEW_BYTES: u64 = 4 * 1024 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetKind {
    Original,
    Preview,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidAsset;

pub fn valid_reference(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            }
        })
}
pub fn resolve_owned_file(
    root: &Path,
    reference: &str,
    kind: AssetKind,
) -> Result<PathBuf, InvalidAsset> {
    if !root.is_absolute() || !valid_reference(reference) {
        return Err(InvalidAsset);
    }
    let root_info = fs::symlink_metadata(root).map_err(|_| InvalidAsset)?;
    if !root_info.is_dir() || root_info.file_type().is_symlink() {
        return Err(InvalidAsset);
    }
    let canonical_root = root.canonicalize().map_err(|_| InvalidAsset)?;
    let (suffix, maximum) = match kind {
        AssetKind::Original => ("original", MAX_ORIGINAL_BYTES),
        AssetKind::Preview => ("preview.jpg", MAX_PREVIEW_BYTES),
    };
    let candidate = canonical_root.join(format!("{reference}.{suffix}"));
    let info = fs::symlink_metadata(&candidate).map_err(|_| InvalidAsset)?;
    if !info.is_file() || info.file_type().is_symlink() || info.len() == 0 || info.len() > maximum {
        return Err(InvalidAsset);
    }
    let canonical = candidate.canonicalize().map_err(|_| InvalidAsset)?;
    if canonical.parent() != Some(canonical_root.as_path()) {
        return Err(InvalidAsset);
    }
    Ok(canonical)
}
pub fn read_preview(root: &Path, reference: &str) -> Result<Vec<u8>, InvalidAsset> {
    let path = resolve_owned_file(root, reference, AssetKind::Preview)?;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| InvalidAsset)?
        .take(MAX_PREVIEW_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| InvalidAsset)?;
    if bytes.len() as u64 > MAX_PREVIEW_BYTES
        || !bytes.starts_with(&[0xff, 0xd8])
        || !bytes.ends_with(&[0xff, 0xd9])
    {
        return Err(InvalidAsset);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    const REFERENCE: &str = "01234567-89ab-4cde-8fab-0123456789ab";
    fn directory() -> PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "pocket-receipt-security-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        path
    }
    #[test]
    fn references_are_opaque_and_cannot_address_paths() {
        assert!(valid_reference(REFERENCE));
        for input in [
            "",
            "../secrets",
            "/data/private",
            "content://camera/1",
            "01234567-89AB-4cde-8fab-0123456789ab",
            "01234567-89ab-4cde-8fab-0123456789ab?x",
        ] {
            assert!(!valid_reference(input));
        }
    }
    #[test]
    fn preview_is_bounded_jpeg_and_original_is_separate() {
        let root = directory();
        fs::write(
            root.join(format!("{REFERENCE}.preview.jpg")),
            [0xff, 0xd8, 0xff, 0xd9],
        )
        .unwrap();
        assert_eq!(
            read_preview(&root, REFERENCE).unwrap(),
            [0xff, 0xd8, 0xff, 0xd9]
        );
        assert!(resolve_owned_file(&root, REFERENCE, AssetKind::Original).is_err());
        fs::write(
            root.join(format!("{REFERENCE}.preview.jpg")),
            b"not an image",
        )
        .unwrap();
        assert!(read_preview(&root, REFERENCE).is_err());
        let large = fs::File::create(root.join(format!("{REFERENCE}.preview.jpg"))).unwrap();
        large.set_len(MAX_PREVIEW_BYTES + 1).unwrap();
        assert!(read_preview(&root, REFERENCE).is_err());
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn symlink_assets_cannot_escape_owned_directory() {
        let root = directory();
        let outside = directory();
        let secret = outside.join("private.jpg");
        fs::write(&secret, [0xff, 0xd8, 0xff, 0xd9]).unwrap();
        std::os::unix::fs::symlink(secret, root.join(format!("{REFERENCE}.preview.jpg"))).unwrap();
        assert!(read_preview(&root, REFERENCE).is_err());
        fs::remove_dir_all(root).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}
