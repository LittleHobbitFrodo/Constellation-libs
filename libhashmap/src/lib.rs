//! Provides various `HashMap` and `HashSet` types such as:
//! - `HashMap`/`HashSet` (`Dictionary`?) - Simple hash map/set implementation for use in Constellation
//! - `StaticMap`/`StaticSet` - A `HashMap` that works only with staitic buffer
//!   - Requires a special procedural macro to generate the buffer with initial data
//!   - **Usecase**: `#![no_std]` and/or embedded environments with memory constraints
//! - `ConstMap`/`ConstSet` - A read-only hash map/set that cannot be appended
//!   - Same with the procedural macro
//!   - **USECASE**: `#![no_std]` and/or embedded, data that changes only between software versions and are not needed to be mutable
//!
//!
//! ## Procedural macros
//! Allows only for built-in types (ints, strings, ...) to be able to calculate the hash on compiletime
//!
//! Could also create a random seed that the map will work with
//!
//! Could work like a trait implementation:
//! ```rust
//! static MAP: StaticMap<&'static str, usize> = static_map! {
//!     static SEED: u64 = 0xDEADBEEF;
//!         //  optional, random if not specified
//!     type Key = &'static str;
//!         //  optional?, guessed by the macro if not specified
//!     type Value = usize;
//!         //  optional?, guessed by the macro if not specified
//!
//!     static Data = {
//!         "Daniel" => 3,
//!         "Veronica" => 4,
//!         "Kate" => 1,
//!         //  ...
//!     };
//! };
//! ```


#![no_std]
