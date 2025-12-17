use std::ops::Deref;

use soroban_rpc as rpc;
use stellar_cli::{
    commands::{NetworkRunnable, contract::invoke, txn_result::TxnEnvelopeResult},
    config::{self, UnresolvedContract, network::Network},
    xdr::{Limits, WriteXdr as _},
};

use crate::contract::{NetworkContract, unverified_contract_id, verified_contract_id};

#[derive(clap::Args, Debug, Clone)]
pub struct Args {
    /// Contract ID of registry
    #[arg(
        long,
        visible_alias = "registry-id",
        env = "STELLAR_REGISTRY_CONTRACT_ID"
    )]
    pub registry_contract_id: Option<UnresolvedContract>,
    /// Whether to use the unverfied registry contract
    #[arg(long, visible_alias = "use-unverifed-registry")]
    pub use_unverifed: bool,

    #[command(flatten)]
    pub config: config::Args,
}

impl Deref for Args {
    type Target = config::Args;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl Args {
    pub fn build_invoke_cmd(
        &self,
        slop: &[&str],
        fee: Option<&stellar_cli::fee::Args>,
        view_only: bool,
    ) -> Result<invoke::Cmd, config::Error> {
        Ok(invoke::Cmd {
            contract_id: UnresolvedContract::Resolved(self.contract_id()?),
            slop: slop.iter().map(Into::into).collect(),
            config: self.config.clone(),
            fee: fee.cloned().unwrap_or_default(),
            send: if view_only {
                invoke::Send::No
            } else {
                invoke::Send::Default
            },
            ..Default::default()
        })
    }

    pub async fn invoke(
        &self,
        slop: &[&str],
        fee: Option<&stellar_cli::fee::Args>,
        view_only: bool,
    ) -> Result<String, invoke::Error> {
        match self
            .build_invoke_cmd(slop, fee, view_only)?
            .run_against_rpc_server(
                Some(&stellar_cli::commands::global::Args::default()),
                Some(&self.config),
            )
            .await?
            .to_envelope()
        {
            TxnEnvelopeResult::TxnEnvelope(transaction_envelope) => {
                println!("{}", transaction_envelope.to_xdr_base64(Limits::none())?);
                std::process::exit(1);
            }
            TxnEnvelopeResult::Res(res) => Ok(res),
        }
    }
}

impl NetworkContract for Args {
    fn contract_id(&self) -> Result<stellar_strkey::Contract, config::Error> {
        let Network {
            network_passphrase, ..
        } = &self.get_network()?;
        self.registry_contract_id.as_ref().map_or_else(
            || {
                Ok(if self.use_unverifed {
                    unverified_contract_id(network_passphrase)
                } else {
                    verified_contract_id(network_passphrase)
                })
            },
            |contract| Ok(contract.resolve_contract_id(&self.locator, network_passphrase)?),
        )
    }

    async fn invoke_registry(
        &self,
        slop: &[&str],
        fee: Option<&stellar_cli::fee::Args>,
        view_only: bool,
    ) -> Result<String, invoke::Error> {
        self.invoke(slop, fee, view_only).await
    }

    async fn view_registry(&self, slop: &[&str]) -> Result<String, invoke::Error> {
        self.invoke(slop, None, true).await
    }

    fn rpc_client(&self) -> Result<rpc::Client, config::Error> {
        Ok(rpc::Client::new(&self.config.get_network()?.rpc_url)?)
    }
}
