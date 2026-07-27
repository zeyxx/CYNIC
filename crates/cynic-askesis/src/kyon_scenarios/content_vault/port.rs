use super::{ContentEnvelope, ContentReference};

pub trait ContentVaultPort {
    type Error;

    fn observe_envelope(
        &self,
        content_ref: &ContentReference,
    ) -> Result<ContentEnvelope, Self::Error>;
}
