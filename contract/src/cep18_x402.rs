use odra::casper_types::bytesrepr::Bytes;
use odra::casper_types::PublicKey;
use odra::casper_types::U256;
use odra::prelude::*;
use odra_modules::cep18_token::Cep18;

use crate::errors::Error;
use crate::events::TransferWithAuthorization;

/// Build the 159-byte authorization message pre-image.
///
/// Layout:
///   15 bytes  — b"casper-x402-v1:"
///   32 bytes  — from account hash
///   32 bytes  — to account hash
///   32 bytes  — amount (U256, little-endian)
///    8 bytes  — valid_after (u64, little-endian)
///    8 bytes  — valid_before (u64, little-endian)
///   32 bytes  — nonce
/// = 159 bytes total
pub fn build_message(
    from_hash: &[u8; 32],
    to_hash: &[u8; 32],
    amount: &U256,
    valid_after: u64,
    valid_before: u64,
    nonce: &[u8],
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(159);
    msg.extend_from_slice(b"casper-x402-v1:");
    msg.extend_from_slice(from_hash);
    msg.extend_from_slice(to_hash);
    let mut amount_bytes = [0u8; 32];
    amount.to_little_endian(&mut amount_bytes);
    msg.extend_from_slice(&amount_bytes);
    msg.extend_from_slice(&valid_after.to_le_bytes());
    msg.extend_from_slice(&valid_before.to_le_bytes());
    // Pad or truncate nonce to 32 bytes
    let mut nonce_padded = [0u8; 32];
    let len = nonce.len().min(32);
    nonce_padded[..len].copy_from_slice(&nonce[..len]);
    msg.extend_from_slice(&nonce_padded);
    msg
}

/// CEP-18 token extended with EIP-3009-style transfer_with_authorization.
#[odra::module(events = [TransferWithAuthorization], errors = Error)]
pub struct Cep18X402 {
    token: SubModule<Cep18>,
    used_nonces: Mapping<(Address, Bytes), bool>,
}

#[odra::module]
impl Cep18X402 {
    pub fn init(&mut self, symbol: String, name: String, decimals: u8, initial_supply: U256) {
        self.token.init(symbol, name, decimals, initial_supply);
    }

    pub fn transfer_with_authorization(
        &mut self,
        from: Address,
        to: Address,
        amount: U256,
        valid_after: u64,
        valid_before: u64,
        nonce: Bytes,
        public_key: PublicKey,
        signature: Bytes,
    ) {
        // 1. Replay protection
        if self
            .used_nonces
            .get(&(from, nonce.clone()))
            .unwrap_or(false)
        {
            self.env().revert(Error::NonceAlreadyUsed);
        }

        // 2. block_time() returns milliseconds — convert to seconds
        let now_secs = self.env().get_block_time() / 1000;

        // 3. Check valid_after
        if now_secs <= valid_after {
            self.env().revert(Error::AuthorizationNotYetValid);
        }

        // 4. Check valid_before
        if now_secs >= valid_before {
            self.env().revert(Error::AuthorizationExpired);
        }

        // 5. Verify that public_key matches the `from` address
        let derived_address = Address::from(public_key.clone());
        if derived_address != from {
            self.env().revert(Error::InvalidPublicKey);
        }

        // 6. Build message and verify signature        
        let message = build_message(
            &from.value(),
            &to.value(),
            &amount,
            valid_after,
            valid_before,
            &nonce,
        );
        let message_bytes = Bytes::from(message);

        if !self
            .env()
            .verify_signature(&message_bytes, &signature, &public_key)
        {
            self.env().revert(Error::InvalidSignature);
        }

        // 7. Mark nonce as used
        self.used_nonces.set(&(from, nonce.clone()), true);

        // 8. Execute transfer (raw_transfer takes refs)
        self.token.raw_transfer(&from, &to, &amount);

        // 9. Emit event
        self.env().emit_event(TransferWithAuthorization {
            from,
            to,
            amount,
            nonce,
        });
    }

    // Delegate standard CEP-18 entry points
    delegate! {
        to self.token {
            fn name(&self) -> String;
            fn symbol(&self) -> String;
            fn decimals(&self) -> u8;
            fn total_supply(&self) -> U256;
            fn balance_of(&self, address: &Address) -> U256;
            fn allowance(&self, owner: &Address, spender: &Address) -> U256;
            fn approve(&mut self, spender: &Address, amount: &U256);
            fn decrease_allowance(&mut self, spender: &Address, decr_by: &U256);
            fn increase_allowance(&mut self, spender: &Address, inc_by: &U256);
            fn transfer(&mut self, recipient: &Address, amount: &U256);
            fn transfer_from(&mut self, owner: &Address, recipient: &Address, amount: &U256);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use odra::casper_types::account::AccountHash;
    use odra::casper_types::bytesrepr::Bytes;
    use odra::host::{Deployer, HostEnv};

    const TOKEN_NAME: &str = "X402Token";
    const TOKEN_SYMBOL: &str = "X402";
    const TOKEN_DECIMALS: u8 = 6;
    const INITIAL_SUPPLY: u64 = 10_000_000;

    struct TestSetup {
        env: HostEnv,
        contract: Cep18X402HostRef,
        sender: Address,
        recipient: Address,
    }

    fn setup() -> TestSetup {
        let env = odra_test::env();
        let sender = env.get_account(0);
        let recipient = env.get_account(1);
        env.advance_block_time(1000000000000);

        let contract = Cep18X402::deploy(
            &env,
            Cep18X402InitArgs {
                symbol: TOKEN_SYMBOL.to_string(),
                name: TOKEN_NAME.to_string(),
                decimals: TOKEN_DECIMALS,
                initial_supply: INITIAL_SUPPLY.into(),
            },
        );

        TestSetup {
            env,
            contract,
            sender,
            recipient,
        }
    }

    fn make_nonce() -> Vec<u8> {
        vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ]
    }

    #[test]
    fn transfer_with_authorization_succeeds() {
        let TestSetup {
            env,
            mut contract,
            sender,
            recipient,
        } = setup();

        let amount = U256::from(1_000u64);
        let nonce = make_nonce();
        let valid_after = 0u64;
        let valid_before = u64::MAX;

        let public_key = env.public_key(&sender);
        let from_hash = AccountHash::from(&public_key);
        let from = Address::Account(from_hash);
        let to = recipient;

        let to_hash = match to.as_account_hash() {
            Some(h) => *h,
            None => AccountHash([0u8; 32]),
        };

        let message = build_message(
            &from_hash.value(),
            &to_hash.0,
            &amount,
            valid_after,
            valid_before,
            &nonce,
        );
        let message_bytes = Bytes::from(message);
        let signature = env.sign_message(&message_bytes, &sender);

        let initial_balance = contract.balance_of(&from);

        contract.transfer_with_authorization(
            from,
            to,
            amount,
            valid_after,
            valid_before,
            nonce.into(),
            public_key,
            signature,
        );

        assert_eq!(contract.balance_of(&from), initial_balance - amount);
        assert_eq!(contract.balance_of(&to), amount);
    }

    #[test]
    fn replay_attack_fails() {
        let TestSetup {
            env,
            mut contract,
            sender,
            recipient,
        } = setup();

        let amount = U256::from(100u64);
        let nonce = make_nonce();
        let valid_after = 0u64;
        let valid_before = u64::MAX;

        let public_key = env.public_key(&sender);
        let from_hash = AccountHash::from(&public_key);
        let from = Address::Account(from_hash);
        let to = recipient;

        let to_hash = match to.as_account_hash() {
            Some(h) => *h,
            None => AccountHash([0u8; 32]),
        };

        let message = build_message(
            &from_hash.value(),
            &to_hash.0,
            &amount,
            valid_after,
            valid_before,
            &nonce,
        );
        let message_bytes = Bytes::from(message);
        let signature = env.sign_message(&message_bytes, &sender);

        // First call succeeds
        contract.transfer_with_authorization(
            from,
            to,
            amount,
            valid_after,
            valid_before,
            nonce.clone().into(),
            public_key.clone(),
            signature.clone(),
        );

        // Second call with same nonce must fail
        let result = contract.try_transfer_with_authorization(
            from,
            to,
            amount,
            valid_after,
            valid_before,
            nonce.into(),
            public_key,
            signature,
        );
        assert!(result.is_err());
    }

    #[test]
    fn expired_authorization_fails() {
        let TestSetup {
            env,
            mut contract,
            sender,
            recipient,
        } = setup();

        let amount = U256::from(100u64);
        let nonce = make_nonce();
        let valid_after = 0u64;
        let valid_before = 0u64; // already expired

        let public_key = env.public_key(&sender);
        let from_hash = AccountHash::from(&public_key);
        let from = Address::Account(from_hash);
        let to = recipient;

        let to_hash = match to.as_account_hash() {
            Some(h) => *h,
            None => AccountHash([0u8; 32]),
        };

        let message = build_message(
            &from_hash.value(),
            &to_hash.0,
            &amount,
            valid_after,
            valid_before,
            &nonce,
        );
        let message_bytes = Bytes::from(message);
        let signature = env.sign_message(&message_bytes, &sender);

        let result = contract.try_transfer_with_authorization(
            from,
            to,
            amount,
            valid_after,
            valid_before,
            nonce.into(),
            public_key,
            signature,
        );
        assert!(result.is_err());
    }

    #[test]
    fn wrong_signature_fails() {
        let TestSetup {
            env,
            mut contract,
            sender,
            recipient,
        } = setup();

        let amount = U256::from(100u64);
        let nonce = make_nonce();
        let valid_after = 0u64;
        let valid_before = u64::MAX;

        let public_key = env.public_key(&sender);
        let from_hash = AccountHash::from(&public_key);
        let from = Address::Account(from_hash);
        let to = recipient;

        // Sign a different message to get a wrong signature
        let bad_message = Bytes::from(b"this is not the right message".as_slice());
        let bad_signature = env.sign_message(&bad_message, &sender);

        let result = contract.try_transfer_with_authorization(
            from,
            to,
            amount,
            valid_after,
            valid_before,
            nonce.into(),
            public_key,
            bad_signature,
        );
        assert!(result.is_err());
    }
}
