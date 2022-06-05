#[derive(Debug)]
pub enum Error {
    Feedback((u32, String)),
    Message(String),
}

impl From<String> for Error {
    fn from(str: String) -> Error {
        Error::Message(str)
    }
}

impl<'a> From<&'a str> for Error {
    fn from(str: &'a str) -> Error {
        Error::Message(str.to_string())
    }
}

impl std::error::Error for Error {}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Error::Feedback((proto_id, msg)) => {
                write!(f, "Feedback {{ proto_id={},msg={} }}", proto_id, msg)
            }
            Error::Message(msg) => write!(f, "Message {{ {} }}", msg),
        }
    }
}
