//! Pure analytics over usage events: block detection, windowing, aggregation,
//! pricing, and burn-rate projection. No I/O; every function takes `now` so the
//! logic is deterministic and unit-testable.

pub mod aggregate;
pub mod blocks;
pub mod burn_rate;
pub mod pricing;
pub mod windows;
