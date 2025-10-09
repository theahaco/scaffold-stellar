use proc_macro2::TokenStream;
use sha2::{Digest, Sha256};

use stellar_build::Network;
use stellar_xdr::curr as xdr;
use xdr::WriteXdr;

use quote::{format_ident, quote};

pub fn parse_asset(str: &str) -> Result<(xdr::Asset, String), xdr::Error> {
    if str == "native" || str == "xlm" {
        return Ok((xdr::Asset::Native, str.to_string()));
    }
    let split: Vec<&str> = str.splitn(2, ':').collect();
    assert!(split.len() == 2, "invalid asset \"{str}\"");
    let code = split[0];
    let issuer: xdr::AccountId = split[1].parse()?;
    let re = regex::Regex::new("^[[:alnum:]]{1,12}$").expect("regex failed");
    assert!(re.is_match(code), "invalid asset \"{str}\"");
    let asset_code: xdr::AssetCode = code.parse()?;
    Ok((
        match asset_code {
            xdr::AssetCode::CreditAlphanum4(asset_code) => {
                xdr::Asset::CreditAlphanum4(xdr::AlphaNum4 { asset_code, issuer })
            }
            xdr::AssetCode::CreditAlphanum12(asset_code) => {
                xdr::Asset::CreditAlphanum12(xdr::AlphaNum12 { asset_code, issuer })
            }
        },
        code.to_string(),
    ))
}

pub fn generate_asset_id(
    asset: &str,
    network: &Network,
) -> Result<(stellar_strkey::Contract, String), xdr::Error> {
    let (asset, code) = parse_asset(asset).unwrap();
    let network_id = xdr::Hash(network.id().into());
    let preimage = xdr::HashIdPreimage::ContractId(xdr::HashIdPreimageContractId {
        network_id,
        contract_id_preimage: xdr::ContractIdPreimage::Asset(asset.clone()),
    });
    let preimage_xdr = preimage.to_xdr(xdr::Limits::none())?;
    Ok((
        stellar_strkey::Contract(Sha256::digest(preimage_xdr).into()),
        code,
    ))
}

/// Generate the code to read the STELLAR_NETWORK environment variable
/// and call the generate_asset_id function
pub fn parse_literal(lit_str: &syn::LitStr, network: &Network) -> TokenStream {
    let (contract_id, code) = generate_asset_id(&lit_str.value(), network).unwrap();
    // let contract_id = format_ident!("\"{contract_id}\"");
    let contract_id = contract_id.to_string();
    let mod_name = format_ident!("{code}");
    quote! {
        #[allow(non_upper_case_globals)]
        pub(crate) mod #mod_name {
            use super::*;
            pub fn client<'a>(env: &soroban_sdk::Env) -> soroban_sdk::token::StellarAssetClient<'a> {
                let asset_address = Address::from_str(&env, #contract_id);
                soroban_sdk::token::StellarAssetClient::new(&env, &asset_address)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use Network::*;
    const NETWORKS: [Network; 4] = [
        Network::Local,
        Network::Testnet,
        Network::Futurenet,
        Network::Mainnet,
    ];

    const USDC: &str = "USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN";

    // Test for  parsing natve token
    #[test]
    fn parse_native() {
        let (asset, code) = parse_asset("native").unwrap();
        assert_eq!(asset, xdr::Asset::Native);
        assert_eq!(code, "native");
        let (asset, code) = parse_asset("xlm").unwrap();
        assert_eq!(asset, xdr::Asset::Native);
        assert_eq!(code, "xlm");
        for network in &NETWORKS {
            match (
                network,
                generate_asset_id("native", network)
                    .unwrap()
                    .0
                    .to_string()
                    .as_str(),
            ) {
                (Local, "CDMLFMKMMD7MWZP3FKUBZPVHTUEDLSX4BYGYKH4GCESXYHS3IHQ4EIG4")
                | (Testnet, "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC")
                | (Futurenet, "CCLFEEF3IHRPZVGCYCRLKQNEXM5XM2BKENUHHXUL45Z4T5WRML3KB4SS")
                | (Mainnet, "CAS3J7GYLGXMF6TDJBBYYSE3HQ6BBSMLNUQ34T6TZMYMW2EVH34XOWMA") => {}
                (x, s) => panic!("Unexpected network {x:?} with asset {s}"),
            }
        }
    }

    // Test for parsing USDC token
    #[test]
    fn parse_usdc() {
        for network in &NETWORKS {
            let asset_id = generate_asset_id(USDC, network).unwrap().0;
            match (network, asset_id.to_string().as_str()) {
                (Local, "CB5SYISL2JCNQQRPFS5H4EFEESWUSNTDYMUNQX7TWZE45MYWYEYWCHAU")
                | (Testnet, "CA2E53VHFZ6YSWQIEIPBXJQGT6VW3VKWWZO555XKRQXYJ63GEBJJGHY7")
                | (Futurenet, "CAIGIAHVHIPSS2OAFGYBPKUG2DFU5AIFE5FZM24UBVCWFJIQZNYXKPE7")
                | (Mainnet, "CCW67TSZV3SSS2HXMBQ5JFGCKJNXKZM7UQUWUZPUTHXSTZLEO7SJMI75") => {}
                (x, s) => panic!("Unexpected network {x:?} with asset {s}"),
            }
        }
    }

    #[test]
    fn native_client() {
        let lit: syn::LitStr = syn::parse_quote!("native");
        let expected = quote! {
            pub(crate) mod native {
                pub fn client<'a>(env: &soroban_sdk::Env) -> soroban_sdk::token::StellarAssetClient<'a> {
                    let asset_address = Address::from_str(&env, "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC");
                    soroban_sdk::token::StellarAssetClient::new(&env, &asset_address)
                }
            }
        };
        let generated = parse_literal(&lit, &Network::Testnet);
        assert_eq!(generated.to_string(), expected.to_string());
    }

    #[test]
    fn xlm_client() {
        let lit: syn::LitStr = syn::parse_quote!("xlm");
        let expected = quote! {
            pub(crate) mod xlm {
                pub fn client<'a>(env: &soroban_sdk::Env) -> soroban_sdk::token::StellarAssetClient<'a> {
                    let asset_address = Address::from_str(&env, "CDLZFC3SYJYDZT7K67VZ75HPJVIEUVNIXF47ZG2FB2RMQQVU2HHGCYSC");
                    soroban_sdk::token::StellarAssetClient::new(&env, &asset_address)
                }
            }
        };
        let generated = parse_literal(&lit, &Network::Testnet);
        assert_eq!(generated.to_string(), expected.to_string());
    }

    #[test]
    fn usdc_client() {
        let lit: syn::LitStr =
            syn::parse_quote!("USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN");
        let expected = quote! {
            pub(crate) mod USDC {
                pub fn client<'a>(env: &soroban_sdk::Env) -> soroban_sdk::token::StellarAssetClient<'a> {
                    let asset_address = Address::from_str(&env, "CA2E53VHFZ6YSWQIEIPBXJQGT6VW3VKWWZO555XKRQXYJ63GEBJJGHY7");
                    soroban_sdk::token::StellarAssetClient::new(&env, &asset_address)
                }
            }
        };
        let generated = parse_literal(&lit, &Network::Testnet);
        assert_eq!(generated.to_string(), expected.to_string());
    }
}
