use crate::domain::packages::PackageCommandContext;
use crate::domain::principal::PrincipalCommandContext;
use crate::domain::privilege::PrivilegeCommandContext;
use crate::domain::services::ServiceCommandContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCommandContext {
    pub principal: PrincipalCommandContext,
    pub privilege: PrivilegeCommandContext,
    pub packages: PackageCommandContext,
    pub services: ServiceCommandContext,
}

impl HostCommandContext {
    pub fn from_current_runtime() -> Self {
        Self {
            principal: PrincipalCommandContext::from_current_runtime(),
            privilege: PrivilegeCommandContext::from_current_runtime(),
            packages: PackageCommandContext::from_current_runtime(),
            services: ServiceCommandContext::from_current_runtime(),
        }
    }
}
