mod de;
mod error;
mod ser;

///////////////////////////////////////////////
pub mod types;
pub use error::{Error, Result};
pub use ser::{to_bytes, to_buffer, Serializer};
pub use de::{from_bytes, Deserializer};