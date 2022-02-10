//! A module to handle all errors
//(from https://github.com/tafia/quick-protobuf.git)
/// An error enum
#[derive(Debug)]
pub enum Error {
    /// Io error
    Io(std::io::Error),
    /// Utf8 Error
    Utf8(::core::str::Utf8Error),
    /// Deprecated feature (in protocol buffer specification)
    Deprecated(&'static str),
    /// Unknown wire type
    UnknownWireType(u8),
    /// Varint decoding error
    Varint(&'static str),
    /// Error while parsing protocol buffer message
    Message(String),
    /// Out of data when reading from or writing to a byte buffer
    UnexpectedEndOfBuffer,
    /// The supplied output buffer is not large enough to serialize the message
    OutputBufferTooSmall(usize, usize, usize),
}

/// A wrapper for `Result<T, Error>`
pub type Result<T> = ::core::result::Result<T, Error>;

impl Into<std::io::Error> for Error {
    fn into(self) -> ::std::io::Error {
        match self {
            Error::Io(x) => x,
            Error::Utf8(x) => std::io::Error::new(std::io::ErrorKind::InvalidData, x),
            x => std::io::Error::new(std::io::ErrorKind::Other, x),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<::core::str::Utf8Error> for Error {
    fn from(e: ::core::str::Utf8Error) -> Error {
        Error::Utf8(e)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => Some(e),
            Error::Utf8(e) => Some(e),
            _ => None,
        }
    }
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{}", e),
            Error::Utf8(e) => write!(f, "{}", e),
            Error::Deprecated(feature) => write!(f, "Feature '{}' has been deprecated", feature),
            Error::UnknownWireType(e) => {
                write!(f, "Unknown wire type '{}', must be less than 6", e)
            }
            Error::Varint(msg) => write!(f, "Cannot decode varint '{}' ", msg),
            Error::Message(msg) => write!(f, "Error while parsing message: {}", msg),
            Error::UnexpectedEndOfBuffer => write!(f, "Unexpected end of buffer"),
            Error::OutputBufferTooSmall(cursor, add, cap) => write!(
                f,
                "Output buffer too small: cursor: {}, add: {}, cap: {}",
                cursor, add, cap
            ),
        }
    }
}
