//! Flecs is a fast and lightweight Entity Component System that lets you build games and simulations with millions of entities.
//!
//! This library provides a comprehensive and low-overhead Rust binding for [Flecs](https://github.com/SanderMertens/flecs) an ECS written in C.
//!
//! ## Documentation
//!
//! - The **[flecs.dev](https://www.flecs.dev/)** website contains comprehensive documentation on Flecs on its features & how to use it for Rust and other languages.
//! - **[Component Macro](component_macro/index.html)** - Complete guide to the `#[derive(Component)]` macro and all its attributes.
//! - **[DSL Macro](dsl/index.html)** - Query, system, and observer DSL documentation
//!
//! ## Safety
//!
//! This crate enables additional runtime checks by default to preserve Rust's
//! borrowing and concurrency guarantees when calling into the underlying C
//! Flecs library. Those checks are provided by the `flecs_safety_locks` feature
//! (enabled by default) and help prevent unsafe aliasing and concurrent mutable
//! access across Flecs callbacks, systems and queries. The bookkeeping is
//! tracked per pipeline stage on the Rust side, so multithreaded systems pay
//! no atomic or cross-thread synchronization cost for these checks, and
//! parallel access to the same component on disjoint entities (including
//! sparse components) is not reported as a conflict.
//!
//! These safety checks imposes a runtime cost. If you fully understand the
//! characteristics of your application and need maximum performance,
//! you may disable `flecs_safety_locks` (e.g. for a Release). Disabling it will
//! improve throughput but removes the runtime protections and may lead to
//! undefined behavior if the API is used in an unsafe way. This might or might not matter
//! depending on the application.

//this is commented since `no_std` is not ready yet
//#![cfg_attr(not(feature = "std"), no_std)] // Enable `no_std` if `std` feature is disabled
#![allow(dead_code)]
#![allow(clippy::module_inception)]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(all(
    feature = "flecs_force_build_release_c",
    feature = "flecs_force_build_debug_c"
))]
compile_error!(
    "Features 'flecs_force_build_release_c' and 'flecs_force_build_debug_c' cannot be enabled at the same time."
);

#[cfg(not(feature = "std"))]
const _: () = panic!("no_std is not ready yet");

#[cfg(feature = "std")]
extern crate std;

#[macro_use]
extern crate alloc;

pub use flecs_ecs_derive as macros;
pub use flecs_ecs_sys as sys;

pub mod core;

pub mod addons;

/// Flecs Rust DSL documentation and examples.
///
/// This module contains comprehensive documentation for the Flecs Rust DSL, including:
/// - Query syntax and operators
/// - System and observer macros
/// - Complete working examples
///
/// All code examples in this module are tested with `cargo test --doc`.
///
/// See the [dsl module documentation](dsl/index.html) for the full guide.
pub mod dsl;

/// Component derive macro documentation and usage guide.
///
/// This module contains comprehensive documentation for the `Component` derive macro, including:
/// - Basic component registration
/// - Component traits (Transitive, Sparse, etc.)
/// - Hooks (on_add, on_set, on_remove, on_replace)
/// - Add and set attributes
/// - Meta information
/// - Common patterns and best practices
pub mod component_macro;

/// this is to allow using the proc macro's inside lib itself that implements its own traits.
extern crate self as flecs_ecs;

pub mod prelude;
#[cfg(test)]
mod tests {
    mod crash_handler {
        /// Use the crash handler for unit tests
        #[cfg(feature = "test-with-crash-handler")]
        #[ctor::ctor]
        fn register_test_crash_handler() {
            test_crash_handler::register();
        }
    }
}
