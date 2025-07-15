use crate::types::{Error, TransportType};
use std::fmt::Debug;

pub trait Peer: Debug + Send + Sync {
    fn kind(&self) -> TransportType;
    fn read(&self) -> Result<Vec<u8>, Error>;
    fn write(&self, data: Vec<u8>) -> Result<(), Error>;
}
