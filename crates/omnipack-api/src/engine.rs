//! The engine calls behind every endpoint, job and drop file: limits, the
//! learned model, and the two request formats.

use crate::erp::{self, ErpRequest, ErpResponse};
use omnipack_core::{pack, PackRequest, PackResult, Ranker};
use omnipack_opt::{optimize, OptimizeOptions, OptimizeResult, Progress};
use serde::Serialize;
use std::sync::atomic::AtomicBool;

/// An error with the HTTP status it maps to.
#[derive(Debug, Clone, Serialize)]
pub struct ApiError {
    #[serde(skip)]
    pub status: u16,
    pub error: String,
}

impl ApiError {
    pub fn bad(msg: impl Into<String>) -> Self {
        ApiError { status: 400, error: msg.into() }
    }
    pub fn too_large(msg: impl Into<String>) -> Self {
        ApiError { status: 413, error: msg.into() }
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        ApiError { status: 404, error: msg.into() }
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        ApiError { status: 500, error: msg.into() }
    }
}

/// Shared settings of the engine calls.
#[derive(Debug, Clone)]
pub struct Engine {
    /// Learned placement weights for `fill: learned` / `FillBias::Learned`,
    /// also offered to the search.
    pub ranker: Option<Ranker>,
    /// Longest search a job or drop file may ask for, seconds (synchronous
    /// requests pass their own, shorter cap).
    pub max_search_s: f64,
    /// Most units in one request.
    pub max_units: usize,
}

impl Default for Engine {
    fn default() -> Self {
        Engine { ranker: None, max_search_s: 600.0, max_units: 20_000 }
    }
}

/// What an engine call reports while a search runs.
pub type OnProgress<'a> = &'a mut dyn FnMut(&Progress);

impl Engine {
    fn check(&self, req: &mut PackRequest) -> Result<(), ApiError> {
        let units: usize = req.items.iter().map(|i| i.quantity as usize).sum();
        if units > self.max_units {
            return Err(ApiError::too_large(format!("{units} units in one request; the limit is {}", self.max_units)));
        }
        if req.options.ranker.is_none() {
            req.options.ranker = self.ranker.clone();
        }
        Ok(())
    }

    fn search_options(o: OptimizeOptions, budget_ms: u64, max_s: f64) -> OptimizeOptions {
        OptimizeOptions { budget_ms: budget_ms.min((max_s * 1000.0) as u64).max(500), ..o }
    }

    /// One pass of the placer.
    pub fn pack(&self, mut req: PackRequest) -> Result<PackResult, ApiError> {
        self.check(&mut req)?;
        pack(&req).map_err(|e| ApiError::bad(e.to_string()))
    }

    /// The ★ Best search, its budget capped at `max_s` seconds.
    pub fn optimize(&self, mut req: PackRequest, options: OptimizeOptions, max_s: f64, cancel: &AtomicBool, progress: OnProgress) -> Result<OptimizeResult, ApiError> {
        self.check(&mut req)?;
        let budget = options.budget_ms;
        let o = Self::search_options(options, budget, max_s.min(self.max_search_s));
        optimize(&req, &o, cancel, progress).map_err(|e| ApiError::bad(e.to_string()))
    }

    /// A simple-format request: one pack, or a search (at most `max_s`
    /// seconds) when it asks for `best`.
    pub fn erp(&self, req: &ErpRequest, max_s: f64, cancel: &AtomicBool, progress: OnProgress) -> Result<(ErpResponse, PackResult), ApiError> {
        let max_s = max_s.min(self.max_search_s);
        let plan = req.to_plan(max_s, self.ranker.as_ref()).map_err(ApiError::bad)?;
        let mut request = plan.request;
        self.check(&mut request)?;
        let result = match plan.search_ms {
            Some(ms) => {
                let o = Self::search_options(OptimizeOptions::default(), ms, max_s);
                let mut r = optimize(&request, &o, cancel, progress).map_err(|e| ApiError::bad(e.to_string()))?;
                r.solutions.remove(0).result
            }
            None => pack(&request).map_err(|e| ApiError::bad(e.to_string()))?,
        };
        let resp = erp::response(req.reference.clone(), &req.units, &result).map_err(ApiError::bad)?;
        Ok((resp, result))
    }
}
