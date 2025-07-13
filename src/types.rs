use std::fmt;

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new<T: Into<String>>(msg: T) -> Self {
        Error {
            message: msg.into(),
        }
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
