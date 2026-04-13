use anyhow::{anyhow, Context, Result};
use casper_types::{bytesrepr::Bytes, AsymmetricType, PublicKey};
use cep18_x402::cep18_x402::Cep18X402HostRef;
use odra::{host::HostRef, prelude::Address};
use x402_eip712::Address as Eip712Address;

/// Handles on-chain settlement of x402 payments on Casper Network.
pub struct CasperSettler {
    x402_token_address: Address,
}

impl CasperSettler {
    pub fn from_env() -> Result<Self> {
        Ok(CasperSettler {
            x402_token_address: std::env::var("X402_TOKEN_ADDRESS")
                .context("Missing X402_TOKEN_ADDRESS env var")?
                .parse()
                .ok()
                .context("Invalid X402_TOKEN_ADDRESS format")?,
        })
    }

    /// Submit a `transfer_with_authorization` call to the Casper network.
    pub async fn call_transfer_with_authorization(
        &self,
        from: Eip712Address,
        to: Eip712Address,
        value: [u8; 32],
        valid_after: [u8; 32],
        valid_before: [u8; 32],
        nonce: [u8; 32],
        public_key_hex: String,
        signature_hex: String,
    ) -> Result<String> {
        let address = self.x402_token_address;
        let from_str = x402_eip712::format_casper_address(&from);
        let to_str = x402_eip712::format_casper_address(&to);
        let amount = odra::casper_types::U256::from_big_endian(&value);
        let valid_after = odra::casper_types::U256::from_big_endian(&valid_after);
        let valid_before = odra::casper_types::U256::from_big_endian(&valid_before);
        println!(
            "Calling transfer_with_authorization with: from={}, to={}, amount={}",
            from_str, to_str, amount
        );
        let sig_bytes = hex::decode(&signature_hex).context("Invalid signature hex")?;
        tokio::task::spawn_blocking(move || {
            let env = odra_casper_livenet_env::env();
            env.set_gas(2_500_000_000);
            let mut token = Cep18X402HostRef::new(address, env);
            let result = token.try_transfer_with_authorization(
                from_str.parse().ok().context("Invalid from address")?,
                to_str.parse().ok().context("Invalid to address")?,
                amount,
                valid_after,
                valid_before,
                Bytes::from(nonce.to_vec()),
                PublicKey::from_hex(&public_key_hex)?,
                sig_bytes.into(),
            );
            if let Err(e) = result {
                eprintln!("Error calling transfer_with_authorization: {:?}", e);
                return Err(anyhow!("Contract call failed: {:?}", e));
            }

            println!("Successfully called transfer_with_authorization");
            Ok("real-tx-hash-placeholder".to_string())
        })
        .await?
    }
}
