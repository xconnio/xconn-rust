use crate::sync::peer::joiner::WebSocketJoiner;
use crate::sync::session::Session;
use crate::types::{JSONSerializerSpec, WSSerializerSpec};
use wampproto::authenticators::anonymous::AnonymousAuthenticator;
use wampproto::authenticators::authenticator::ClientAuthenticator;

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
