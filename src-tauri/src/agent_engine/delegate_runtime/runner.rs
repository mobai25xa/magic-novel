use async_trait::async_trait;

use crate::mission::delegate_types::{DelegateRequest, DelegateResult};
use crate::mission::role_profile::RoleProfile;
use crate::models::AppError;

#[derive(Debug, Clone)]
pub struct DelegateRunContext {
    pub request: DelegateRequest,
    pub role_profile: RoleProfile,
    pub project_path: String,
    pub mission_dir: String,
    pub mission_id: String,
    pub actor_id: String,
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub api_key: String,
}

impl DelegateRunContext {
    pub fn normalized(mut self) -> Self {
        self.request = self.request.normalized();
        self.role_profile = self.role_profile.normalized();
        self.project_path = self.project_path.trim().to_string();
        self.mission_dir = self.mission_dir.trim().to_string();
        self.mission_id = self.mission_id.trim().to_string();
        self.actor_id = self.actor_id.trim().to_string();
        self.provider = self.provider.trim().to_string();
        self.model = self.model.trim().to_string();
        self.base_url = self.base_url.trim().to_string();
        self.api_key = self.api_key.trim().to_string();
        self
    }
}

#[async_trait]
pub trait DelegateRunner: Send + Sync {
    fn runner_kind(&self) -> &'static str;

    async fn run_delegate(&self, context: DelegateRunContext) -> Result<DelegateResult, AppError>;
}
