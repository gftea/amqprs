use amqp_serde::types::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct ProtocolName(Octect, Octect, Octect, Octect);

#[derive(Serialize, Deserialize, Debug)]
struct ProtocolVersion {
    major: Octect,
    minor: Octect,
    revision: Octect,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProtocolHeader {
    name: ProtocolName,
    id: Octect,
    version: ProtocolVersion,
}
impl ProtocolHeader {
    pub fn set_version(&mut self, major: Octect, minor: Octect, revision: Octect) {
        self.version = ProtocolVersion {
            major,
            minor,
            revision,
        };
    }
}
impl Default for ProtocolHeader {
    fn default() -> Self {
        Self {
            name: ProtocolName(b'A', b'M', b'Q', b'P'),
            id: 0,
            version: ProtocolVersion {
                major: 0,
                minor: 9,
                revision: 1,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use amqp_serde::{from_bytes, to_bytes};

    use super::ProtocolHeader;

    #[test]
    fn test_serialize() {
        let data = ProtocolHeader::default();
        let frame = to_bytes(&data);
        println!("{frame:?}");
    }

    #[test]
    fn test_deserialize() {
        let data = [65, 77, 81, 80, 0, 0, 9, 0];
        let frame: ProtocolHeader = from_bytes(&data).unwrap();
        println!("{frame:?}");
    }
}
