//! Every path the server answers. Caddy's site config (`deploy/botc.caddy`)
//! must route each of these to the server; `deploy/tests/files_agree.sh`
//! checks that.

pub const HEALTH: &str = "/health";
pub const ME: &str = "/api/me";
pub const SCRIPTS: &str = "/api/scripts";
pub const ROOMS: &str = "/api/rooms";
pub const JOIN: &str = "/api/rooms/{code}/join";
pub const PLAY: &str = "/ws/play";
pub const BRIDGE: &str = "/ws/bridge";
