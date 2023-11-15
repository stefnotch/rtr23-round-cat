use std::path::Path;

pub trait AssetProcessor {
    /// Rough filtering for files.
    /// Concrete checks are done later.
    fn can_potentially_handle(&self, path: &Path) -> bool;
}
