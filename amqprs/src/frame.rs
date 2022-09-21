use amqp_serde::types::*;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug)]
struct ProtocolName(Octect,Octect,Octect,Octect);

#[derive(Serialize, Deserialize, Debug)]
struct ProtocolVersion {
    major: Octect,
    minor: Octect,
    revision: Octect,
}


#[derive(Serialize, Deserialize, Debug)]
struct ProtocolHeader {
    name: ProtocolName,
    id: Octect,
    version: ProtocolVersion,
}
impl ProtocolHeader {
    pub fn new() -> Self {
        Self {
            name: ProtocolName(b'A', b'M', b'Q', b'P'),
            id: 0,
            version: ProtocolVersion {
                major: 0,
                minor: 9,
                revision: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use amqp_serde::{to_bytes,from_bytes};

    use super::ProtocolHeader;

    #[test]
    fn test_serialize() {
        let data = ProtocolHeader::new();
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
