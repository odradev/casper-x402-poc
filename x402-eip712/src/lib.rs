#![cfg_attr(not(test), no_std)]

pub use casper_eip_712::Address;

extern crate alloc;

/// Tag byte for a Casper account-hash address.
pub const ACCOUNT_TAG: u8 = 0x00;
/// Tag byte for a Casper contract (package) hash address.
pub const CONTRACT_TAG: u8 = 0x01;

/// Create a Casper `Address` from raw 32-byte account-hash bytes (tag byte `0x00`).
pub fn casper_address_from_bytes(account_hash_bytes: [u8; 32]) -> Address {
    casper_address_from_parts(ACCOUNT_TAG, account_hash_bytes)
}

/// Create a Casper `Address` from raw 32-byte contract-hash bytes (tag byte `0x01`).
pub fn casper_contract_address_from_bytes(contract_hash_bytes: [u8; 32]) -> Address {
    casper_address_from_parts(CONTRACT_TAG, contract_hash_bytes)
}

/// Create a Casper `Address` from a tag byte and 32 raw bytes.
pub fn casper_address_from_parts(tag: u8, hash_bytes: [u8; 32]) -> Address {
    let mut arr = [0u8; 33];
    arr[0] = tag;
    arr[1..].copy_from_slice(&hash_bytes);
    Address::Casper(arr)
}

/// Extract the raw 32-byte hash from a Casper `Address` (account or contract).
///
/// Accepts tag bytes `0x00` (account) and `0x01` (contract).
pub fn casper_address_to_bytes(addr: &Address) -> Result<[u8; 32], &'static str> {
    match addr {
        Address::Casper(b) if b[0] == ACCOUNT_TAG || b[0] == CONTRACT_TAG => {
            let mut bytes = [0u8; 32];
            bytes.copy_from_slice(&b[1..]);
            Ok(bytes)
        }
        Address::Casper(_) => Err("invalid Casper address: unexpected tag byte"),
        Address::Eth(_) => Err("expected Casper address, got Eth"),
    }
}

/// Return the tag byte of a Casper `Address`.
pub fn casper_address_tag(addr: &Address) -> Result<u8, &'static str> {
    match addr {
        Address::Casper(b) if b[0] == ACCOUNT_TAG || b[0] == CONTRACT_TAG => Ok(b[0]),
        Address::Casper(_) => Err("invalid Casper address: unexpected tag byte"),
        Address::Eth(_) => Err("expected Casper address, got Eth"),
    }
}

/// Format a Casper `Address` as a human-readable string:
/// - `0x00` (account) → `account-hash-{hex}`
/// - `0x01` (contract) → `hash-{hex}`
///
/// Only available outside wasm32 (requires `alloc::format`).
#[cfg(not(target_arch = "wasm32"))]
pub fn format_casper_address(addr: &Address) -> alloc::string::String {
    match addr {
        Address::Casper(b) => {
            let hex = b[1..]
                .iter()
                .fold(alloc::string::String::new(), |mut s, byte| {
                    s.push_str(&alloc::format!("{:02x}", byte));
                    s
                });
            match b[0] {
                ACCOUNT_TAG => alloc::format!("account-hash-{}", hex),
                CONTRACT_TAG => alloc::format!("hash-{}", hex),
                tag => alloc::format!("unknown-{:02x}-{}", tag, hex),
            }
        }
        Address::Eth(b) => {
            let hex = b.iter().fold(alloc::string::String::new(), |mut s, byte| {
                s.push_str(&alloc::format!("{:02x}", byte));
                s
            });
            alloc::format!("0x{}", hex)
        }
    }
}
/// Build the EIP-712 domain separator for x402.
pub fn x402_domain(
    chain_id: &str,
    x402_token_address: [u8; 32],
) -> casper_eip_712::DomainSeparator {
    casper_eip_712::DomainBuilder::new()
        .name("Cep18x402")
        .version("1")
        .custom_field(
            "chain_id",
            casper_eip_712::DomainFieldValue::String(chain_id.into()),
        )
        .custom_field(
            "contract_package_hash",
            casper_eip_712::DomainFieldValue::Bytes32(x402_token_address),
        )
        .build()
}

/// EIP-3009/x402-style transfer authorization for Casper.
#[cfg_attr(
    not(target_arch = "wasm32"),
    derive(Debug, Clone, serde::Serialize, serde::Deserialize)
)]
pub struct TransferWithAuthorization {
    #[cfg_attr(not(target_arch = "wasm32"), serde(with = "serde_address"))]
    pub from: Address, // Key::AccountHash
    #[cfg_attr(not(target_arch = "wasm32"), serde(with = "serde_address"))]
    pub to: Address, // Key::AccountHash
    pub value: [u8; 32],        // U256
    #[cfg_attr(not(target_arch = "wasm32"), serde(rename = "validAfter"))]
    pub valid_after: [u8; 32],  // U256 (timestamp)
    #[cfg_attr(not(target_arch = "wasm32"), serde(rename = "validBefore"))]
    pub valid_before: [u8; 32], // U256 (timestamp)
    pub nonce: [u8; 32],
}

impl casper_eip_712::Eip712Struct for TransferWithAuthorization {
    fn type_string() -> &'static str {
        "TransferAuthorization(address from,address to,uint256 value,uint256 valid_after,uint256 valid_before,bytes32 nonce)"
    }

    fn encode_data(&self) -> alloc::vec::Vec<u8> {
        let mut data = alloc::vec::Vec::with_capacity(6 * 32);
        data.extend(casper_eip_712::encode_address(self.from));
        data.extend(casper_eip_712::encode_address(self.to));
        data.extend(casper_eip_712::encode_uint256(self.value));
        data.extend(casper_eip_712::encode_uint256(self.valid_after));
        data.extend(casper_eip_712::encode_uint256(self.valid_before));
        data.extend(casper_eip_712::encode_bytes32(self.nonce));
        data
    }
}

/// Serde support for `casper_eip_712::Address` (not available on wasm32).
#[cfg(not(target_arch = "wasm32"))]
pub mod serde_address {
    use casper_eip_712::Address;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Serialise as a hex string.  Eth = 20 bytes, Casper = 33 bytes.
    pub fn serialize<S: Serializer>(addr: &Address, s: S) -> Result<S::Ok, S::Error> {
        let bytes: alloc::vec::Vec<u8> = match addr {
            Address::Eth(b) => b.to_vec(),
            Address::Casper(b) => b.to_vec(),
        };
        let hex = bytes
            .iter()
            .fold(alloc::string::String::new(), |mut acc, b| {
                acc.push_str(&alloc::format!("{:02x}", b));
                acc
            });
        hex.serialize(s)
    }

    /// Deserialise from a hex string.  Length determines the variant.
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Address, D::Error> {
        let hex = alloc::string::String::deserialize(d)?;
        let bytes = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(serde::de::Error::custom))
            .collect::<Result<alloc::vec::Vec<u8>, _>>()?;
        match bytes.len() {
            20 => {
                let mut arr = [0u8; 20];
                arr.copy_from_slice(&bytes);
                Ok(Address::Eth(arr))
            }
            33 => {
                let mut arr = [0u8; 33];
                arr.copy_from_slice(&bytes);
                Ok(Address::Casper(arr))
            }
            n => Err(serde::de::Error::custom(alloc::format!(
                "invalid address length {n}: expected 20 (Eth) or 33 (Casper)"
            ))),
        }
    }
}
