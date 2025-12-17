use sha2::{Digest, Sha256};
use soroban_rpc as rpc;
use stellar_cli::{
    commands::contract::invoke, config, xdr::{self, WriteXdr}
};
use stellar_strkey::ed25519::PublicKey;

pub const REGISTRY_NAME: &str = "registry";

pub trait NetworkContract {
    fn contract_id(&self) -> Result<stellar_strkey::Contract, config::Error>;

    fn contract_sc_address(&self) -> Result<xdr::ScAddress, config::Error> {
        Ok(xdr::ScAddress::Contract(xdr::ContractId(xdr::Hash(
            self.contract_id()?.0,
        ))))
    }

    fn invoke_registry(
        &self,
        slop: &[&str],
        fee: Option<&stellar_cli::fee::Args>,
        view_only: bool,
    ) -> impl std::future::Future<Output = Result<String, invoke::Error>> + Send;

    fn view_registry(
        &self,
        slop: &[&str],
    ) -> impl std::future::Future<Output = Result<String, invoke::Error>> + Send;

    fn rpc_client(&self) -> Result<rpc::Client, config::Error>;
}

pub type Salt = [u8; 32];

pub trait ToSalt {
    fn to_salt(&self) -> Salt;
}

struct ContractId {
    salt: Salt,
    deployer: stellar_strkey::ed25519::PublicKey,
}

impl ToSalt for &str {
    fn to_salt(&self) -> Salt {
        Sha256::digest(self.as_bytes()).into()
    }
}

impl ContractId {
    pub fn new<T: ToSalt>(deployer: PublicKey, salt: &T) -> Self {
        Self {
            salt: salt.to_salt(),
            deployer,
        }
    }

    pub fn id(&self, network_passphrase: &str) -> stellar_strkey::Contract {
        let network_id = xdr::Hash(Sha256::digest(network_passphrase.as_bytes()).into());
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

pub fn stellar_address() -> stellar_strkey::ed25519::PublicKey {
    "GAMPJROHOAW662FINQ4XQOY2ULX5IEGYXCI4SMZYE75EHQBR6PSTJG3M"
        .parse()
        .unwrap()
}

pub fn contract_id(network_passphrase: &str, salt: &str) -> stellar_strkey::Contract {
    ContractId::new(stellar_address(), &salt).id(network_passphrase)
}

pub fn verified_contract_id(network_passphrase: &str) -> stellar_strkey::Contract {
    contract_id(network_passphrase, "verified")
}

pub fn unverified_contract_id(network_passphrase: &str) -> stellar_strkey::Contract {
    contract_id(network_passphrase, "unverified")
}

#[cfg(test)]
mod generate_id {
    use stellar_cli::config::network::passphrase::*;

    fn test_contract_id((passphrase, contract_id): (&str, &str)) {
        assert_eq!(
            &super::verified_contract_id(passphrase).to_string(),
            contract_id,
            "{passphrase}"
        );
    }
    #[test]
    fn futurenet() {
        test_contract_id((
            FUTURENET,
            "CBUP2U7IY4GBZWILAGFGBOGEJEVSWZ6FAIKAX2L7PYOEE7R556LNXRJM",
        ));
    }

    #[test]
    fn testnet() {
        test_contract_id((
            TESTNET,
            "CBFFTTX7QKA76FS4LHHQG54BC7JF5RMEX4RTNNJ5KEL76LYHVO3E3OEE",
        ));
    }

    #[test]
    fn mainnet() {
        test_contract_id((
            MAINNET,
            "CCRKU6NT4CRG4TVKLCCJFU7EOSAUBHWGBJF2JWZJSKTJTXCXXTKOJIUS",
        ));
    }

    #[test]
    fn local() {
        test_contract_id((
            LOCAL,
            "CDUK4O7FPAPZWAMS6PBKM7E4IO5MCBJ2ZPZ6K2GOHK33YW7Q4H7YZ35Z",
        ));
    }
}
