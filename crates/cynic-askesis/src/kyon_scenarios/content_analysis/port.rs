use super::{AnalysisCapability, ContentAnalysisRequest, ContentAnalysisResult};

pub trait ContentAnalysisPort {
    type Error;

    fn capability(&self) -> AnalysisCapability;

    fn analyze(
        &self,
        request: &ContentAnalysisRequest,
    ) -> Result<ContentAnalysisResult, Self::Error>;
}
