use std::error::Error;
use std::fmt;

pub trait Protocol {
    type Outbound: ?Sized;
    type Inbound;

    fn input(buffer: &[u8], max_package_size: usize) -> Result<Option<usize>, ProtocolError>;
    fn encode(data: &Self::Outbound) -> Result<Vec<u8>, ProtocolError>;
    fn decode(packet: &[u8]) -> Result<Self::Inbound, ProtocolError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    InvalidData(&'static str),
    InvalidNumber,
    PackageTooLarge { actual: usize, maximum: usize },
    UnexpectedEnd,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidData(message) => formatter.write_str(message),
            Self::InvalidNumber => formatter.write_str("protocol contains an invalid number"),
            Self::PackageTooLarge { actual, maximum } => {
                write!(
                    formatter,
                    "package size {actual} exceeds configured maximum {maximum}"
                )
            }
            Self::UnexpectedEnd => formatter.write_str("protocol packet ended unexpectedly"),
        }
    }
}

impl Error for ProtocolError {}

pub(crate) fn enforce_package_size(
    package_size: usize,
    maximum: usize,
) -> Result<(), ProtocolError> {
    if package_size > maximum {
        return Err(ProtocolError::PackageTooLarge {
            actual: package_size,
            maximum,
        });
    }
    Ok(())
}
