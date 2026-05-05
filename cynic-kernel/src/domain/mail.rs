//! MailPort — domain contract for organism email identity.
//! DORMANT: declared but not yet implemented. Stub satisfies the compiler.

use async_trait::async_trait;

/// Domain port for email operations (send, receive, check inbox).
#[async_trait]
pub trait MailPort: Send + Sync + std::fmt::Debug {}
