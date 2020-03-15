#[path = "../../target/riko/bridge.rs"]
mod bridge;

pub mod android;
pub mod pki;
pub(crate) mod util;

/// The protagonist.
pub struct Node;

impl Node {
    /// Constructor.
    ///
    /// No need to explicitly start running the client. Once it is created, everything is functional
    /// until the whole object is dropped.
    pub fn create() -> Node {
        Self
    }
}
