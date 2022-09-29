use crate::types::{Octect, ShortUint};

pub const FRAME_HEADER_SIZE: usize = 7;

pub const FRAME_METHOD: Octect = 1;
pub const FRAME_HEADER: Octect = 2;
pub const FRAME_BODY: Octect = 3;
pub const FRAME_HEARTBEAT: Octect = 8;


pub const FRAME_END: Octect = 206;

// all reply code are unsigned 16bit integer
pub const REPLY_SUCCESS: ShortUint = 200; //This reply code is reserved for future use

// soft error for channel
pub const CONTENT_TOO_LARGE: ShortUint = 311;
pub const NO_ROUTE: ShortUint = 312;
pub const NO_CONSUMERS: ShortUint = 313;
pub const ACCESS_REFUSED: ShortUint = 403;
pub const NOT_FOUND: ShortUint = 404;
pub const RESOURCE_LOCKED: ShortUint = 405;
pub const PRECONDITION_FAILED: ShortUint = 406;

// hard error for connection
pub const CONNECTION_FORCED: ShortUint = 320;
pub const INVALID_PATH: ShortUint = 402;
pub const FRAME_ERROR: ShortUint = 501;
pub const SYNTAX_ERROR: ShortUint = 502;
pub const COMMAND_INVALID: ShortUint = 503;
pub const CHANNEL_ERROR: ShortUint = 504;
pub const UNEXPECTED_FRAME: ShortUint = 505;
pub const RESOURCE_ERROR: ShortUint = 506;
pub const NOT_ALLOWED: ShortUint = 530;
pub const NOT_IMPLEMENTED: ShortUint = 540;
pub const INTERNAL_ERROR: ShortUint = 541;

// class id
pub const CLASS_CONNECTION: ShortUint = 10;
pub const CLASS_CHANNEL: ShortUint = 20;
pub const CLASS_EXCHANGE: ShortUint = 40;
pub const CLASS_QUEUE: ShortUint = 50;
pub const CLASS_BASIC: ShortUint = 60;
pub const CLASS_TX: ShortUint = 90;