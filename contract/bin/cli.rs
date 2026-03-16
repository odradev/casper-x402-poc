use cep18_x402::cep18_x402::{Cep18X402, Cep18X402InitArgs};
use odra_cli::{cspr, DeployerExt};

fn main() {
    odra_cli::OdraCli::new()
        .about("X402 CLI")
        .deploy(Deploy)
        .build()
        .run();
}

struct Deploy;

impl odra_cli::deploy::DeployScript for Deploy {
    fn deploy(
        &self,
        env: &odra::host::HostEnv,
        container: &mut odra_cli::DeployedContractsContainer,
    ) -> core::result::Result<(), odra_cli::deploy::Error> {
        Cep18X402::load_or_deploy(
            &env,
            Cep18X402InitArgs {
                symbol: "X402".to_string(),
                name: "Casper X402 Token".to_string(),
                decimals: 2,
                initial_supply: 1_000_000_000.into(),
            },
            container,
            cspr!(550),
        )?;

        Ok(())
    }
}
