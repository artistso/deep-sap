//! Independently testable numerical core for DEEP SAP.
//!
//! The executable retains the Bevy ECS simulation. This library target keeps
//! geometry and polynomial regression tests isolated from the heavyweight game
//! binary linker so mathematical regressions remain fast and deterministic.

pub mod math;
