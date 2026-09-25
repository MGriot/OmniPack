//! The single source of numeric tolerances (millimetres).

/// Two surfaces closer than this are considered touching (contact detection).
pub const CONTACT: f64 = 0.05;

/// Interpenetration deeper than this is a collision. Touching faces are allowed.
pub const PENETRATION: f64 = 0.01;

/// Slack for "fits inside the container" checks.
pub const BOUNDS: f64 = 1e-6;

/// A contact normal must have at least this downward component to carry weight.
pub const MIN_SUPPORT_NORMAL_Y: f64 = 0.05;

/// A support polygon thinner than this is really a line (or a point): contact
/// manifolds on curved surfaces scatter slightly around the true contact line.
pub const DEGENERATE_WIDTH: f64 = 1.0;
