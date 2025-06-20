use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct PatchMeta {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    pub requires_android: Option<String>,
    pub conflicts_with: Option<Vec<String>>,
    pub author: Option<String>,
}

pub fn load_patch_meta<P: AsRef<std::path::Path>>(patch_path: P) -> Option<PatchMeta> {
    let manifest_path = patch_path.as_ref().join("patch.yaml");
    if !manifest_path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&manifest_path).ok()?;
    serde_yaml::from_str(&content).ok()
}
