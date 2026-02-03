/*
[INPUT]:  API schema definitions and serde requirements
[OUTPUT]: Typed Rust structs/enums with serialization support
[POS]:    Data layer - type definitions for API communication
[UPDATE]: When API schema changes or new types added
*/

pub mod enums;
pub mod models;
pub mod requests;
pub mod responses;

pub use enums::*;
pub use models::*;
pub use requests::*;
pub use responses::*;
