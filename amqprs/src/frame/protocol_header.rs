use amqp_serde::types::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct ProtocolName(Octect, Octect, Octect, Octect);

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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

    use crate::frame::protocol_header::{ProtocolName, ProtocolVersion};

    use super::ProtocolHeader;

    #[test]
    fn test_serialize() {
        let data = ProtocolHeader::default();
        let frame = to_bytes(&data).unwrap();
        assert_eq!([65, 77, 81, 80, 0, 0, 9, 1].to_vec(), frame);
    }

    #[test]
    fn test_deserialize() {
        let data = [65, 77, 81, 80, 0, 0, 9, 1];
        let frame: ProtocolHeader = from_bytes(&data).unwrap();
        let ProtocolHeader { name, id, version } = frame;
        assert_eq!(ProtocolName(b'A', b'M', b'Q', b'P'), name);
        assert_eq!(0, id);
        assert_eq!(
            ProtocolVersion {
                major: 0,
                minor: 9,
                revision: 1
            },
            version
        );
    }
}
