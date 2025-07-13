use crate::types::Error;
use std::fmt::Debug;

pub type TransportType = usize;
pub const TRANSPORT_WEB_SOCKET: TransportType = 1;
pub const TRANSPORT_RAW_SOCKET: TransportType = 2;

pub trait Peer: Debug + Send + Sync {
    fn kind(&self) -> TransportType;
    fn read(&self) -> Result<Vec<u8>, Error>;
    fn write(&self, data: Vec<u8>) -> Result<(), Error>;
}
