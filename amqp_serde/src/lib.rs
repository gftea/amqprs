mod de;
mod error;
mod ser;

///////////////////////////////////////////////
pub mod types;
pub use de::{from_bytes, Deserializer};
pub use error::{Error, Result};
pub use ser::{to_buffer, to_bytes, Serializer};
