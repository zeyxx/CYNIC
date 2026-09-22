//! Application service: centralized model store + HF CLI.

use crate::adapters::hf::HfCliAdapter;
use crate::adapters::local::FsModelStore;
use crate::domain::model_store::ModelStoreReport;
use crate::infra::model_layout::ModelLayout;
use crate::ports::hf_hub_port::{
    HfCliStatus, HfDownloadReport, HfDownloadRequest, HfHubError, HfHubPort,
};
use crate::ports::local_runtime_port::LocalRuntimeError;
use crate::ports::model_store_port::ModelStorePort;

#[derive(Debug)]
pub struct ModelStoreService {
    layout: ModelLayout,
    store: FsModelStore,
    hf: HfCliAdapter,
}

impl ModelStoreService {
    pub fn from_env() -> Self {
        let layout = ModelLayout::from_env();
        let store = FsModelStore::with_layout(layout.clone());
        let hf = HfCliAdapter::from_layout(&layout);
        Self { layout, store, hf }
    }

    pub fn layout(&self) -> &ModelLayout {
        &self.layout
    }

    pub fn scan(&self) -> Result<ModelStoreReport, LocalRuntimeError> {
        self.store.scan()
    }

    pub fn scan_and_persist(&self) -> Result<ModelStoreReport, LocalRuntimeError> {
        let report = self.store.scan()?;
        self.store.persist_registry(&report)?;
        Ok(report)
    }

    pub fn hf_status(&self) -> Result<HfCliStatus, HfHubError> {
        self.hf.status(&self.layout)
    }

    pub fn hf_download(&self, req: &HfDownloadRequest) -> Result<HfDownloadReport, HfHubError> {
        let report = self.hf.download(&self.layout, req)?;
        // Refresh registry after successful vault download
        if report.ok {
            let _ = self.scan_and_persist();
        }
        Ok(report)
    }

    pub fn layout_ref(&self) -> &ModelLayout {
        &self.layout
    }
}
