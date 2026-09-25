//! Geometry layer for OmniPack.
//!
//! Conventions (see `docs/conventions.md`): **Y is up**, X is container width,
//! Z is container depth with the door at `z = depth`. Lengths are millimetres,
//! masses kilograms. Placement positions are the minimum corner of the item's
//! world-space axis-aligned bounding box.

pub mod hull2d;
pub mod query;
pub mod shape;
pub mod tol;

pub use hull2d::{
    clip_convex, convex_hull, polygon_area, ray_exit_distance, signed_distance_to_polygon, Pt2,
};
pub use parry3d_f64 as parry;
pub use parry3d_f64::na;
pub use query::{drop_height, floor_contacts, overlaps, support_contacts, Body, SupportContact};
pub use shape::{Orientation, OrientedShape, RenderMesh, Shape};
