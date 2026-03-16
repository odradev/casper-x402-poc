use cep18_x402::cep18_x402::{Cep18X402, Cep18X402InitArgs};
use odra::{host::{Deployer, InstallConfig}, prelude::Addressable};

fn main() {
    let address_file_path = std::env::var("X402_CONTRACT_ADDRESS_FILE").expect("Address file path must be set in X402_CONTRACT_ADDRESS_FILE env var");
    // Check if the contract is already deployed by looking for the address file
    if std::path::Path::new(&address_file_path).exists() {
        let address_str = std::fs::read_to_string(&address_file_path).expect("Failed to read contract address from file");
        println!("Contract already deployed at address: {}", address_str);
        return;
    }

    let env = odra_casper_livenet_env::env();

    env.set_gas(500_000_000_000);
    let contract = Cep18X402::try_deploy_with_cfg(
        &env, 
        Cep18X402InitArgs { 
            symbol: "X402".to_string(), 
            name: "Casper X402 Token".to_string(), 
            decimals: 2, 
            initial_supply: 1_000_000_000.into()  
        },
        InstallConfig { package_named_key: "Cep18X403".to_string(), is_upgradable: true, allow_key_override: true }
    );
    let contract = contract.expect("Failed to deploy contract");
    std::fs::write(&address_file_path, contract.address().to_string()).expect("Failed to write contract address to file");

    // Verify deployment by reading the address back and creating a host reference
    let address_str = std::fs::read_to_string(&address_file_path).expect("Failed to read contract address from file");
    println!("Deployed contract address: {}", address_str);
}