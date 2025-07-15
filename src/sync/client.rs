use crate::sync::joiner::WebSocketJoiner;
use crate::sync::session::Session;
use crate::types::{CBORSerializerSpec, Error, JSONSerializerSpec, WSSerializerSpec};

use wampproto::authenticators::anonymous::AnonymousAuthenticator;
use wampproto::authenticators::authenticator::ClientAuthenticator;
use wampproto::authenticators::cryptosign::CryptoSignAuthenticator;
use wampproto::authenticators::ticket::TicketAuthenticator;
use wampproto::authenticators::wampcra::WAMPCRAAuthenticator;

pub struct Client {
    serializer: Box<dyn WSSerializerSpec>,
    authenticator: Option<Box<dyn ClientAuthenticator>>,
}

impl Client {
    pub fn new(serializer: Box<dyn WSSerializerSpec>, authenticator: Box<dyn ClientAuthenticator>) -> Self {
        Self {
            serializer,
            authenticator: Some(authenticator),
        }
    }

    pub fn connect(self, uri: &str, realm: &str) -> Session {
        let serializer = self.serializer.serializer().clone();
        let joiner = if self.authenticator.is_none() {
            let authenticator = AnonymousAuthenticator::new("", Default::default());
            WebSocketJoiner::new(self.serializer, Box::new(authenticator))
        } else {
            WebSocketJoiner::new(self.serializer, self.authenticator.clone().unwrap())
        };

        if let Ok((peer, details)) = joiner.join(uri, realm) {
            Session::new(details, peer, serializer)
        } else {
            panic!("failed to join websocket");
        }
    }
}

impl Default for Client {
    fn default() -> Self {
        Self {
            serializer: Box::new(JSONSerializerSpec {}),
            authenticator: None,
        }
    }
}

pub fn connect_anonymous(uri: &str, realm: &str) -> Result<Session, Error> {
    let client = Client::default();
    Ok(client.connect(uri, realm))
}

pub fn connect_ticket(uri: &str, realm: &str, authid: &str, ticket: &str) -> Result<Session, Error> {
    let serializer = Box::new(CBORSerializerSpec {});
    let authenticator = Box::new(TicketAuthenticator::new(authid, ticket, Default::default()));

    let client = Client::new(serializer, authenticator);
    Ok(client.connect(uri, realm))
}

pub fn connect_wampcra(uri: &str, realm: &str, authid: &str, secret: &str) -> Result<Session, Error> {
    let serializer = Box::new(CBORSerializerSpec {});
    let authenticator = Box::new(WAMPCRAAuthenticator::new(authid, secret, Default::default()));

    let client = Client::new(serializer, authenticator);
    Ok(client.connect(uri, realm))
}

pub fn connect_cryptosign(uri: &str, realm: &str, authid: &str, private_key_hex: &str) -> Result<Session, Error> {
    let serializer = Box::new(CBORSerializerSpec {});

    match CryptoSignAuthenticator::try_new(authid, private_key_hex, Default::default()) {
        Ok(authenticator) => {
            let client = Client::new(serializer, Box::new(authenticator));
            Ok(client.connect(uri, realm))
        }

        Err(e) => Err(Error::new(e.to_string())),
    }
}
