//! Core data structures and object system

pub mod object;
pub mod shm;
pub mod message;
pub mod meta;
pub mod parameter;

pub use object::*;
pub use shm::*;
pub use message::*;
pub use meta::*;
pub use parameter::*;
