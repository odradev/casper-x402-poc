#![cfg_attr(not(test), no_std)]

extern crate alloc;
/// Build the EIP-712 domain separator for x402.
pub fn x402_domain(chain_name: &str, x402_token_address: [u8; 32]) -> casper_eip_712::DomainSeparator {
    casper_eip_712::DomainBuilder::new()
        .name("Cep18x402")
        .version("1")
        .custom_field("chain_name", casper_eip_712::DomainFieldValue::String(chain_name.into()))
        .custom_field("contract_package_hash", casper_eip_712::DomainFieldValue::Bytes32(x402_token_address))
        .build()
}


/// EIP-3009/x402-style transfer authorization for Casper.
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, Clone, serde::Serialize, serde::Deserialize))]
pub struct TransferAuthorization {
    pub from: [u8; 32],       // AccountHash
    pub to: [u8; 32],         // AccountHash
    pub value: [u8; 32],      // U256
    pub valid_after: u64,
    pub valid_before: u64,
    pub nonce: [u8; 32],
}

impl casper_eip_712::Eip712Struct for TransferAuthorization {
    fn type_string() -> &'static str {
        "TransferAuthorization(bytes32 from,bytes32 to,uint256 value,uint64 valid_after,uint64 valid_before,bytes32 nonce)"
    }

    fn encode_data(&self) -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec::Vec::with_capacity(6 * 32);
        data.extend(casper_eip_712::encode_bytes32(self.from));
        data.extend(casper_eip_712::encode_bytes32(self.to));
        data.extend(casper_eip_712::encode_uint256(self.value));
        data.extend(casper_eip_712::encode_uint64(self.valid_after));
        data.extend(casper_eip_712::encode_uint64(self.valid_before));
        data.extend(casper_eip_712::encode_bytes32(self.nonce));
        data
    }
}