use stellar_xdr::{Limits, ReadXdr, TransactionEnvelope};

use crate::io_safety::MAX_INPUT_BYTES;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEncoding {
    Base64,
    RawXdr,
}

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("XDR input exceeds 1 MiB")]
    TooLarge,
    #[error("invalid TransactionEnvelope XDR: {0}")]
    Invalid(String),
}

pub fn decode_envelope(
    input: &[u8],
    encoding: InputEncoding,
) -> Result<TransactionEnvelope, DecodeError> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(DecodeError::TooLarge);
    }
    let limits = Limits {
        depth: 64,
        len: MAX_INPUT_BYTES,
    };
    match encoding {
        InputEncoding::Base64 => TransactionEnvelope::from_xdr_base64(input, limits),
        InputEncoding::RawXdr => TransactionEnvelope::from_xdr(input, limits),
    }
    .map_err(|error| DecodeError::Invalid(error.to_string()))
}
