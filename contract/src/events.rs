use odra::casper_types::bytesrepr::Bytes;
use odra::casper_types::U256;
use odra::prelude::*;

#[odra::event]
pub struct TransferWithAuthorization {
    pub from: Address,
    pub to: Address,
    pub amount: U256,
    pub nonce: Bytes,
}
