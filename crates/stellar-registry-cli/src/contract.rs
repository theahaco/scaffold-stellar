use sha2::{Digest, Sha256};
use stellar_cli::xdr::{self, WriteXdr};

pub fn stellar_address() -> stellar_strkey::ed25519::PublicKey {
    "GBMJ2WUAZXELW27JLKTGXM5ZQHS4MXVUPQRDQD47J6UKRYPYUFW64LT5"
        .parse()
        .unwrap()
}

pub fn contract_id(network_passphrase: &str) -> stellar_strkey::Contract {
    let network_id = xdr::Hash(Sha256::digest(network_passphrase.as_bytes()).into());
    let preimage = xdr::HashIdPreimage::ContractId(xdr::HashIdPreimageContractId {
        network_id,
        contract_id_preimage: xdr::ContractIdPreimage::Address(
            xdr::ContractIdPreimageFromAddress {
                address: xdr::ScAddress::Account(xdr::AccountId(
                    xdr::PublicKey::PublicKeyTypeEd25519(stellar_address().0.into()),
                )),
                salt: xdr::Uint256([0; 32]),
            },
        ),
    });
    let preimage_xdr = preimage
        .to_xdr(xdr::Limits::none())
        .expect("HashIdPreimage should not fail encoding to xdr");
    stellar_strkey::Contract(Sha256::digest(preimage_xdr).into())
}
