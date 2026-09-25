//! `backends.toml` loading, env fallbacks and system contract extraction.

pub(crate) mod backend;
mod helpers;
pub(crate) mod storage;

#[cfg(test)]
mod tests;
