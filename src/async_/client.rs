use crate::async_::joiner::{RawSocketJoiner, WebSocketJoiner};
use crate::async_::session::Session;
use crate::common::types::{CBORSerializerSpec, Error, SerializerSpec};

use wampproto::authenticators::anonymous::AnonymousAuthenticator;
use wampproto::authenticators::authenticator::ClientAuthenticator;
use wampproto::authenticators::cryptosign::CryptoSignAuthenticator;
use wampproto::authenticators::ticket::TicketAuthenticator;
use wampproto::authenticators::wampcra::WAMPCRAAuthenticator;

pub struct Client {
    serializer: Box<dyn SerializerSpec>,
    authenticator: Box<dyn ClientAuthenticator>,
}

impl Client {
    pub fn new(serializer: Box<dyn SerializerSpec>, authenticator: Box<dyn ClientAuthenticator>) -> Self {
        Self {
            serializer,
            authenticator,
        }
    }

    pub async fn connect(self, uri: &str, realm: &str) -> Result<Session, Error> {
        if uri.starts_with("ws://") || uri.starts_with("wss://") {
            let serializer = self.serializer.serializer();
            let joiner = WebSocketJoiner::new(self.serializer, self.authenticator);
            let (peer, details) = joiner.join(uri, realm).await.map_err(|e| Error::new(e.to_string()))?;
            Ok(Session::new(details, peer, serializer))
        } else if uri.starts_with("rs://")
            || uri.starts_with("rss://")
            || uri.starts_with("tcp://")
            || uri.starts_with("tcps://")
        {
            let serializer = self.serializer.serializer();
            let joiner = RawSocketJoiner::new(self.serializer, self.authenticator);
            let (peer, details) = joiner.join(uri, realm).await.map_err(|e| Error::new(e.to_string()))?;
            Ok(Session::new(details, peer, serializer))
        } else {
            Err(Error::new("Invalid URI scheme".to_string()))
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self {
            serializer: Box::new(CBORSerializerSpec {}),
            authenticator: Box::new(AnonymousAuthenticator::new("", Default::default())),
        }
    }
}

pub async fn connect_anonymous(uri: &str, realm: &str) -> Result<Session, Error> {
    let client = Client::default();
    client.connect(uri, realm).await
}

pub async fn connect_ticket(uri: &str, realm: &str, authid: &str, ticket: &str) -> Result<Session, Error> {
    let serializer = Box::new(CBORSerializerSpec {});
    let authenticator = Box::new(TicketAuthenticator::new(authid, ticket, Default::default()));

    let client = Client::new(serializer, authenticator);
    client.connect(uri, realm).await
}

pub async fn connect_wampcra(uri: &str, realm: &str, authid: &str, secret: &str) -> Result<Session, Error> {
    let serializer = Box::new(CBORSerializerSpec {});
    let authenticator = Box::new(WAMPCRAAuthenticator::new(authid, secret, Default::default()));

    let client = Client::new(serializer, authenticator);
    client.connect(uri, realm).await
}

pub async fn connect_cryptosign(uri: &str, realm: &str, authid: &str, private_key_hex: &str) -> Result<Session, Error> {
    let serializer = Box::new(CBORSerializerSpec {});
    let authenticator = CryptoSignAuthenticator::try_new(authid, private_key_hex, Default::default())
        .map_err(|e| Error::new(e.to_string()))?;

    let client = Client::new(serializer, Box::new(authenticator));
    client.connect(uri, realm).await
}
