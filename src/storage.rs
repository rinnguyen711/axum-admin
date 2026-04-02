use crate::error::AdminError;
use async_trait::async_trait;
use std::path::PathBuf;
use uuid::Uuid;

#[async_trait]
pub trait FileStorage: Send + Sync {
    /// Store `data` (raw bytes) and return the public URL/path to the stored file.
    /// `original_filename` is used only to preserve the file extension.
    async fn save(&self, original_filename: &str, data: &[u8]) -> Result<String, AdminError>;
    /// Delete a stored file by its URL/path (as returned by `save`). Idempotent.
    async fn delete(&self, url: &str) -> Result<(), AdminError>;
    /// Convert a stored path to a public URL. For `LocalStorage` this is a no-op
    /// (save already returns the URL). Custom backends may differ.
    fn url(&self, path: &str) -> String;
}

pub struct LocalStorage {
    root: PathBuf,
    base_url: String,
}

impl LocalStorage {
    pub fn new(root: impl Into<PathBuf>, base_url: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            base_url: base_url.into(),
        }
    }
}

#[async_trait]
impl FileStorage for LocalStorage {
    async fn save(&self, original_filename: &str, data: &[u8]) -> Result<String, AdminError> {
        let ext = std::path::Path::new(original_filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let uuid_name = if ext.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            format!("{}.{}", Uuid::new_v4(), ext)
        };
        let dest = self.root.join(&uuid_name);
        tokio::fs::write(&dest, data).await.map_err(|e| {
            AdminError::Custom(format!("Upload failed: {e}"))
        })?;
        let base = self.base_url.trim_end_matches('/');
        Ok(format!("{base}/{uuid_name}"))
    }

    async fn delete(&self, url: &str) -> Result<(), AdminError> {
        let base = self.base_url.trim_end_matches('/');
        if !url.starts_with(base) {
            return Err(AdminError::Custom("URL does not belong to this storage".to_string()));
        }
        let filename = url
            .trim_start_matches(base)
            .trim_start_matches('/');
        let path = self.root.join(filename);
        if !path.starts_with(&self.root) {
            return Err(AdminError::Custom("Invalid file path".to_string()));
        }
        match tokio::fs::remove_file(&path).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(AdminError::Custom(format!("Delete failed: {e}"))),
        }
    }

    fn url(&self, path: &str) -> String {
        path.to_string()
    }
}
