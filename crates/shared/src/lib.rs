pub const SERVICE_NAME: &str = "github-human-auth";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RepositoryRef {
    pub owner: String,
    pub name: String,
}
