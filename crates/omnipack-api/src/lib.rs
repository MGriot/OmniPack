//! OmniPack's integration API: the packing engine over HTTP, for ERP systems
//! (SAP and others) and any other program.
//!
//! - REST endpoints under `/api/v1` (see `openapi.json` and docs/api.md):
//!   full OmniPack JSON and a simple ERP format with unit conversion;
//!   synchronous packing, search and validation; CSV load lists.
//! - Background jobs with polling and callback URLs.
//! - File drop folders for systems that exchange files.
//!
//! [`serve`] runs it; the standalone `omnipack-server` and the desktop app's
//! local API both use this crate.

pub mod csv;
pub mod drop;
pub mod engine;
pub mod erp;
pub mod jobs;
mod server;

pub use server::{router, serve, ApiConfig, AppState, ServeHandle, OPENAPI};
