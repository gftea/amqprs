use crate::types::{Octect, ShortUint};

pub const FRAME_METHOD: Octect = 1;
pub const FRAME_HEADER: Octect = 2;
pub const FRAME_BODY: Octect = 3;
pub const FRAME_HEARTBEAT: Octect = 8;

pub const FRAME_END: Octect = 206;

// all reply code are unsigned 16bit integer
// soft error / channel
pub const CONTENT_TOO_LARGE: ShortUint = 311;
pub const NO_ROUTE: ShortUint = 312;
pub const NO_CONSUMERS: ShortUint = 313;
pub const ACCESS_REFUSED: ShortUint = 403;
pub const NOT_FOUND: ShortUint = 404;
pub const RESOURCE_LOCKED: ShortUint = 405;
pub const PRECONDITION_FAILED: ShortUint = 406;

// hard error / connection
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