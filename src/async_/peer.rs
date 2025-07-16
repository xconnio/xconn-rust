use crate::common::types::{Error, TransportType};
use async_trait::async_trait;
use std::fmt::Debug;

#[async_trait]
pub trait Peer: Debug + Send + Sync {
    fn kind(&self) -> TransportType;
    async fn read(&self) -> Result<Vec<u8>, Error>;
    async fn write(&self, data: Vec<u8>) -> Result<(), Error>;
}
