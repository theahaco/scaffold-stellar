use crate::{named_registry::PrefixedName, registry::Registry};
use sha2::{Digest, Sha256};
use soroban_rpc as rpc;
use stellar_build::Network;
use stellar_cli::{
    commands::{NetworkRunnable, contract::invoke, txn_result::TxnEnvelopeResult},
    config::{self, UnresolvedContract},
    xdr::{self, WriteXdr as _},
};
use stellar_strkey::ed25519::PublicKey;

pub struct Contract {
    id: stellar_strkey::Contract,
    config: config::Args,
}

impl Contract {
    pub fn new(id: stellar_strkey::Contract, config: &config::Args) -> Self {
        Self {
            id,
            config: config.clone(),
        }
    }

    pub fn rpc_client(&self) -> Result<rpc::Client, config::Error> {
        Ok(rpc::Client::new(&self.config.get_network()?.rpc_url)?)
    }

    pub fn sc_address(&self) -> xdr::ScAddress {
        xdr::ScAddress::Contract(xdr::ContractId(xdr::Hash(self.id.0)))
    }

    pub fn id(&self) -> stellar_strkey::Contract {
        self.id
    }

    pub fn build_invoke_cmd(
        &self,
        slop: &[&str],
        fee: Option<&stellar_cli::fee::Args>,
        view_only: bool,
    ) -> invoke::Cmd {
        invoke::Cmd {
            contract_id: UnresolvedContract::Resolved(self.id.clone()),
            slop: slop.iter().map(Into::into).collect(),
            config: self.config.clone(),
            fee: fee.cloned().unwrap_or_default(),
            send: if view_only {
                invoke::Send::No
            } else {
                invoke::Send::Default
            },
            ..Default::default()
        }
    }

    pub async fn invoke(
        &self,
        slop: &[&str],
        fee: Option<&stellar_cli::fee::Args>,
        view_only: bool,
    ) -> Result<TxnEnvelopeResult<String>, invoke::Error> {
        Ok(self
            .build_invoke_cmd(slop, fee, view_only)
            .run_against_rpc_server(
                Some(&stellar_cli::commands::global::Args::default()),
                Some(&self.config),
            )
            .await?
            .to_envelope())

        // {
        //     TxnEnvelopeResult::TxnEnvelope(transaction_envelope) => {
        //         println!("{}", transaction_envelope.to_xdr_base64(Limits::none())?);
        //         std::process::exit(1);
        //     }
        //     TxnEnvelopeResult::Res(res) => Ok(res),
        // }
    }

    pub async fn invoke_with_result(
        &self,
        slop: &[&str],
        fee: Option<&stellar_cli::fee::Args>,
        view_only: bool,
    ) -> Result<String, invoke::Error> {
        Ok(self
            .build_invoke_cmd(slop, fee, view_only)
            .run_against_rpc_server(
                Some(&stellar_cli::commands::global::Args::default()),
                Some(&self.config),
            )
            .await?
            .into_result()
            .unwrap())

        // {
        //     TxnEnvelopeResult::TxnEnvelope(transaction_envelope) => {
        //         println!("{}", transaction_envelope.to_xdr_base64(Limits::none())?);
        //         std::process::exit(1);
        //     }
        //     TxnEnvelopeResult::Res(res) => Ok(res),
        // }
    }

    pub(crate) fn config(&self) -> &config::Args {
        &self.config
    }
}

pub trait ToSalt {
    fn into_salt(self) -> Salt;
}

pub type Salt = [u8; 32];

impl ToSalt for &str {
    fn into_salt(self) -> Salt {
        Sha256::digest(self.as_bytes()).into()
    }
}

#[derive(Clone)]
pub enum ContractId {
    Resolved(stellar_strkey::Contract),
    Unresolved(stellar_cli::config::UnresolvedContract),
    PreHash(PreHashContractID),
    FromRegistry(PrefixedName),
}

impl ContractId {
    pub async fn resolve_id(
        &self,
        config: &config::Args,
    ) -> Result<stellar_strkey::Contract, invoke::Error> {
        let network_passphrase = config.get_network()?.network_passphrase;
        Ok(match self {
            ContractId::Resolved(contract) => *contract,
            ContractId::Unresolved(unresolved_contract) => {
                unresolved_contract.resolve_contract_id(&config.locator, &network_passphrase)?
            }
            ContractId::PreHash(pre_hash_contract_id) => {
                pre_hash_contract_id.id(&network_passphrase.parse().unwrap())
            }
            ContractId::FromRegistry(PrefixedName { channel, name }) => {
                Registry::new(config, channel.as_deref())
                    .await?
                    .fetch_contract_id(name)
                    .await?
            }
        })
    }

    pub async fn resolve_contract(
        &self,
        config: &config::Args,
    ) -> Result<Contract, invoke::Error> {
        Ok(Contract::new(self.resolve_id(config).await?, config))
    }
}

#[derive(Clone, Debug)]
pub struct PreHashContractID {
    salt: Salt,
    deployer: stellar_strkey::ed25519::PublicKey,
}

impl PreHashContractID {
    pub fn new<T: ToSalt>(deployer: PublicKey, salt: T) -> Self {
        Self {
            salt: salt.into_salt(),
            deployer,
        }
    }

    pub fn id(&self, network_passphrase: &Network) -> stellar_strkey::Contract {
        let network_id = network_passphrase.id().into();
        let preimage = xdr::HashIdPreimage::ContractId(xdr::HashIdPreimageContractId {
            network_id,
            contract_id_preimage: xdr::ContractIdPreimage::Address(
                xdr::ContractIdPreimageFromAddress {
                    address: xdr::ScAddress::Account(xdr::AccountId(
                        xdr::PublicKey::PublicKeyTypeEd25519(self.deployer.0.into()),
                    )),
                    salt: xdr::Uint256(self.salt),
                },
            ),
        });
        let preimage_xdr = preimage
            .to_xdr(xdr::Limits::none())
            .expect("HashIdPreimage should not fail encoding to xdr");
        stellar_strkey::Contract(Sha256::digest(preimage_xdr).into())
    }
}

// impl NetworkContract for Args {
//     fn contract_id(
//         &self,
//         named_registry: PrefixedName,
//     ) -> Result<stellar_strkey::Contract, config::Error> {
//         let Network {
//             network_passphrase, ..
//         } = &self.get_network()?;
//         self.registry_contract_id.as_ref().map_or_else(
//             || {
//                 Ok(if self.use_unverified {
//                     unverified_contract_id(network_passphrase)
//                 } else {
//                     verified_contract_id(network_passphrase)
//                 })
//             },
//             |contract| Ok(contract.resolve_contract_id(&self.locator, network_passphrase)?),
//         )
//     }

//     async fn invoke_registry(
//         &self,
//         slop: &[&str],
//         fee: Option<&stellar_cli::fee::Args>,
//         view_only: bool,
//     ) -> Result<String, invoke::Error> {
//         self.invoke(slop, fee, view_only).await
//     }

//     async fn view_registry(&self, slop: &[&str]) -> Result<String, invoke::Error> {
//         self.invoke(slop, None, true).await
//     }

//     fn rpc_client(&self) -> Result<rpc::Client, config::Error> {
//         Ok(rpc::Client::new(&self.config.get_network()?.rpc_url)?)
//     }
// }
