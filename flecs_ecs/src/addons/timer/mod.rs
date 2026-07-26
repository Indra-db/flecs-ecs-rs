//! Timers trigger actions at periodic or one-shot intervals.
//!
//! The timer addon provides time-based scheduling for systems and other actions.
//! Timers are synchronous and increment each frame by `delta_time`, making them
//! suitable for game logic that needs to run at specific intervals or after delays.
//!
//! # Features
//!
//! - **Periodic Intervals**: Execute actions repeatedly at fixed intervals
//! - **One-shot Timeouts**: Execute actions once after a delay
//! - **System Integration**: Attach timers to systems for scheduled execution
//! - **Start/Stop Control**: Pause and resume timers dynamically
//! - **Rate Control**: Control system execution frequency
//!
//! # Usage
//!
//! Timers can be applied to systems or standalone entities:
//!
//! ## Interval Timer (Periodic)
//!
//! ```
//! use flecs_ecs::prelude::*;
//!
//! let mut world = World::new();
//!
//! // Create a system that runs every 2 seconds
//! world.system::<()>()
//!     .set_interval(2.0)
//!     .run(|mut it| {
//!         println!("System runs every 2 seconds");
//!     });
//!
//! // Progress the world
//! world.progress();
//! ```
//!
//! ## Timeout Timer (One-shot)
//!
//! ```
//! use flecs_ecs::prelude::*;
//!
//! let mut world = World::new();
//!
//! // Create a system that runs once after 5 seconds
//! let timer_system = world.system::<()>()
//!     .run(|mut it| {
//!         println!("System runs once after 5 seconds");
//!     });
//!
//! // Set timeout on the system entity
//! timer_system.set_timeout(5.0);
//!
//! world.progress();
//! ```
//!
//! ## Timer Control
//!
//! ```
//! use flecs_ecs::prelude::*;
//!
//! let mut world = World::new();
//!
//! let timer = world.system::<()>()
//!     .set_interval(1.0)
//!     .run(|mut it| {
//!         println!("Tick!");
//!     });
//!
//! // Stop the timer
//! timer.stop();
//!
//! // Start it again
//! timer.start();
//! ```
//!
//! # See also
//!
//! - [`TimerAPI`] - Trait providing timer operations
//! - [`System::set_interval()`](crate::addons::system::System::set_interval)
//! - [`System::set_timeout()`](crate::addons::system::System::set_timeout)

mod timer;
pub use timer::*;
mod system;
mod system_builder;
mod world;
