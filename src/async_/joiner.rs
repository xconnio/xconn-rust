use crate::async_::peer::Peer;
use crate::async_::websocket::WebSocketPeer;
use crate::types::{Error, JSONSerializerSpec, SessionDetails, WSSerializerSpec};
use futures_util::StreamExt;
use http::Uri;
use std::str::FromStr;
use tokio_tungstenite::connect_async_with_config;
use tungstenite::ClientRequestBuilder;
use tungstenite::protocol::WebSocketConfig;
use wampproto::authenticators::anonymous::AnonymousAuthenticator;
use wampproto::authenticators::authenticator::ClientAuthenticator;
use wampproto::joiner;
use wampproto::serializers::serializer::Serializer;

pub struct WebSocketJoiner {
    serializer: Box<dyn WSSerializerSpec>,
    authenticator: Box<dyn ClientAuthenticator>,
}

impl Default for WebSocketJoiner {
    fn default() -> Self {
        Self::new(
            Box::new(JSONSerializerSpec {}),
            Box::new(AnonymousAuthenticator::default()),
        )
    }
}

impl WebSocketJoiner {
    pub fn new(serializer: Box<dyn WSSerializerSpec>, authenticator: Box<dyn ClientAuthenticator>) -> Self {
        Self {
            serializer,
            authenticator,
        }
    }

    pub async fn join(&self, uri: &str, realm: &str) -> Result<(Box<dyn Peer>, SessionDetails), Error> {
        let uri = Uri::from_str(uri).unwrap();
        let request = ClientRequestBuilder::new(uri).with_sub_protocol(self.serializer.subprotocol());
        let config = Some(WebSocketConfig::default());

        match connect_async_with_config(request, config, false).await {
            Ok((ws, _)) => {
                let (writer, reader) = ws.split();
                let peer = WebSocketPeer::new(reader, writer, self.serializer.is_binary());
                let auth = self.authenticator.clone();
                join(peer, realm, self.serializer.serializer(), auth).await
            }

            Err(e) => Err(Error::new(format!("failed to connect: {e}"))),
        }
    }
}

pub async fn join(
    peer: Box<dyn Peer>,
    realm: &str,
    serializer: Box<dyn Serializer>,
    authenticator: Box<dyn ClientAuthenticator>,
) -> Result<(Box<dyn Peer>, SessionDetails), Error> {
    let mut proto = joiner::Joiner::new(realm, serializer.clone(), authenticator);
    if let Ok(hello) = proto.send_hello() {
        peer.write(hello).await?;

        loop {
            if let Ok(reply) = peer.read().await {
                match proto.receive(reply) {
                    Ok(Some(to_send)) => {
                        peer.write(to_send).await?;
                    }

                    Ok(None) => {
                        if let Ok(Some(details)) = proto.session_details() {
                            let details = SessionDetails::new(
                                details.id,
                                details.realm.to_string(),
                                details.authid.to_string(),
                                details.auth_role.to_string(),
                            );

                            return Ok((peer, details));
                        }
                    }

                    Err(e) => {
                        return Err(Error::new(format!("failed to join: {e}")));
                    }
                }
            }
        }
    } else {
        Err(Error::new("failed to send hello"))
    }
}
