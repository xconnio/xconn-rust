use std::fmt;
use std::fmt::Debug;
use wampproto::serializers::cbor::CBORSerializer;
use wampproto::serializers::json::JSONSerializer;
use wampproto::serializers::msgpack::MsgPackSerializer;
use wampproto::serializers::serializer::Serializer;

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new<T: Into<String>>(msg: T) -> Self {
        Error { message: msg.into() }
    }
}

// Implement Display so errors print nicely
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MyError: {}", self.message)
    }
}

// Implement the std::error::Error trait
impl std::error::Error for Error {}

#[derive(Debug, Clone)]
pub struct SessionDetails {
    id: i64,
    realm: String,
    authid: String,
    auth_role: String,
}

impl SessionDetails {
    pub fn new(id: i64, realm: String, authid: String, auth_role: String) -> Self {
        Self {
            id,
            realm,
            authid,
            auth_role,
        }
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn realm(&self) -> String {
        self.realm.clone()
    }

    pub fn authid(&self) -> String {
        self.authid.clone()
    }

    pub fn auth_role(&self) -> String {
        self.auth_role.clone()
    }
}

pub trait WSSerializerSpec: Debug + Sync + Send {
    fn subprotocol(&self) -> String;
    fn serializer(&self) -> Box<dyn Serializer>;
    fn is_binary(&self) -> bool;
}

#[derive(Debug, Clone, Default)]
pub struct JSONSerializerSpec;

impl WSSerializerSpec for JSONSerializerSpec {
    fn subprotocol(&self) -> String {
        "wamp.2.json".to_string()
    }

    fn serializer(&self) -> Box<dyn Serializer> {
        Box::new(JSONSerializer {})
    }

    fn is_binary(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, Default)]
pub struct CBORSerializerSpec;

impl WSSerializerSpec for CBORSerializerSpec {
    fn subprotocol(&self) -> String {
        "wamp.2.cbor".to_string()
    }

    fn serializer(&self) -> Box<dyn Serializer> {
        Box::new(CBORSerializer {})
    }

    fn is_binary(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone, Default)]
pub struct MsgPackSerializerSpec;

impl WSSerializerSpec for MsgPackSerializerSpec {
    fn subprotocol(&self) -> String {
        "wamp.2.msgpack".to_string()
    }

    fn serializer(&self) -> Box<dyn Serializer> {
        Box::new(MsgPackSerializer {})
    }

    fn is_binary(&self) -> bool {
        true
    }
}
