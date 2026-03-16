use odra::prelude::*;

#[odra::odra_error]
pub enum Error {
    NonceAlreadyUsed = 1,
    AuthorizationExpired = 2,
    AuthorizationNotYetValid = 3,
    InvalidSignature = 4,
    InvalidFromAddress = 5,
    InvalidPublicKey = 6,
    Debug = 999,
}
