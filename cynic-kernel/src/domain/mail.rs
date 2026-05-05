//! MailPort — domain contract for organism email identity.
//! DORMANT: port trait declared, backend not yet built.

use async_trait::async_trait;

#[async_trait]
pub trait MailPort: Send + Sync + std::fmt::Debug {}
