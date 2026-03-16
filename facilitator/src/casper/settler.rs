use anyhow::{Context, Result, anyhow};
use casper_types::{AsymmetricType, PublicKey, bytesrepr::Bytes};
use cep18_x402::cep18_x402::Cep18X402HostRef;
use odra::{host::HostRef, prelude::Address};

/// Handles on-chain settlement of x402 payments on Casper Network.
///
/// In mock mode this returns a fake transaction hash without touching the network.
/// In real mode it would use casper-client to submit a TransactionV1.
pub struct CasperSettler {
    // host_env: HostEnv,
    x402_token_address: Address,
}

impl CasperSettler {
    pub fn from_env() -> Result<Self> {
        Ok(CasperSettler {
            // host_env: odra_casper_livenet_env::env(),
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
        from: &str,
        to: &str,
        amount: u64,
        valid_after: u64,
        valid_before: u64,
        nonce_hex: &str,
        public_key_hex: &str,
        signature_hex: &str,
    ) -> Result<String> {
        // In a full implementation this would:
        //
        //   1. Load the secret key from self.secret_key_path
        //   2. Build RuntimeArgs with all authorization fields
        //   3. Call TransactionV1Builder::new_targeting_package(...)
        //   4. Submit via casper_client::put_transaction(...)
        //   5. Return the hex-encoded transaction hash
        //
        // For now we return a descriptive placeholder so callers can test the
        // full flow without a live Casper node.
        let address = self.x402_token_address;
        let from = format!("account-hash-{}", from);
        let to = format!("account-hash-{}", to);
        let nonce = hex::decode(nonce_hex).context("Invalid nonce hex")?;
        let public_key_hex = public_key_hex.to_string();
            println!("Calling transfer_with_authorization with: from={}, to={}, amount={}", from, to, amount
        );
        let sig_bytes = hex::decode(signature_hex).context("Invalid signature hex")?;
        tokio::task::spawn_blocking(move || {
            let env = odra_casper_livenet_env::env();
            env.set_gas(2_500_000_000);
            let mut token = Cep18X402HostRef::new(address, env);
            let result = token.try_transfer_with_authorization(
                from.parse().ok().context("Invalid from address")?,
                to.parse().ok().context("Invalid to address")?,
                amount.into(),
                valid_after,
                valid_before,
                Bytes::from(nonce),
                PublicKey::from_hex(&public_key_hex)?,
                sig_bytes.into()
            );
            if let Err(e) = result {
                eprintln!("Error calling transfer_with_authorization: {:?}", e);
                return Err(anyhow!("Contract call failed: {:?}", e));
            }

            println!("Successfully called transfer_with_authorization with");
            Ok("real-tx-hash-placeholder".to_string())
        }).await?
    }
}
