#[macro_use]
extern crate enclose;

pub mod errors;
pub mod rpc;
pub mod shared_queue;
pub mod xi_thread;

pub use crate::errors::ErrorMsg;
pub use crate::rpc::Core;
pub use crate::shared_queue::{CoreMsg, SharedQueue};
pub use crate::xi_thread::XiPeer;
