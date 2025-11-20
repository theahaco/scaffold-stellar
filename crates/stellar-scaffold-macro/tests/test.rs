extern crate std;

use std::io::Cursor;
use std::str::FromStr;
use rust_decimal::prelude::ToPrimitive;
use soroban_sdk::{
    self,
     Address,
    Env,
};
use stellar_xdr::curr as xdr;
use stellar_xdr::curr::{WriteXdr};
use stellar_scaffold_macro::import_asset;

import_asset!("native");

#[test]
pub fn test_macro() {
    let env = &Env::default();
    let admin =&Address::from_str(env, "GC6RVI3M7DM5RFEZVBDLGOC4QJHEW66Q4TCTXGGLNCL7EXR5BM2VWJCG");
    let bob = &Address::from_str(env, "GAMMPSGF3M62GCHVW4JKREODD7LZWQQ4Y7CURTR2M5SVEWOGYQ2XVFJU");
    let addr = native::contract_id(env);
    let _sac = native::stellar_asset_client(env);
    let _client = native::token_client(env);

    // Local network
    assert_eq!(addr.to_string(), to_string(env, "CDMLFMKMMD7MWZP3FKUBZPVHTUEDLSX4BYGYKH4GCESXYHS3IHQ4EIG4"));

    let asset = parse_asset("ASTT:GC6RVI3M7DM5RFEZVBDLGOC4QJHEW66Q4TCTXGGLNCL7EXR5BM2VWJCG");
    let mut data = Vec::new();
    let cursor = Cursor::new(&mut data);
    let mut limit = xdr::Limited::new(cursor, xdr::Limits::none());
    asset.unwrap().0.write_xdr(&mut limit).unwrap();

    println!("{}", data.len());
    let arr: [u8; 44] = data.try_into().unwrap();

    let addr = env.deployer().with_stellar_asset(arr).deploy();

    println!("{:?}", addr);

    let sac = soroban_sdk::token::StellarAssetClient::new(&env, &addr);

    assert_eq!(sac.symbol(), to_string(env,"ASTT"));
    assert_eq!(sac.admin(), *admin);
    env.mock_all_auths();
    sac.mint(&bob, &1_000_000_000.to_i128().unwrap());
    // println!("{}", sac.balance(bob));

    // assert_eq!(client.symbol(), to_string(env,"XLM"));
}

// #[test]
// pub fn foo () {
//     let mut data = Vec::new();
//     let cursor = Cursor::new(&mut data);
//     let mut limit = Limited::new(cursor, Limits::none());
//     let res = Asset::Native.write_xdr(&mut limit);
//     println!("{:?}", res);
//     println!("{:?}", data)
// }

pub fn to_string(env: &Env, s: &str) -> soroban_sdk::String {
    soroban_sdk::String::from_str(env, s)
}


pub fn parse_asset(str: &str) -> Result<(xdr::Asset, String), xdr::Error> {
    if str == "native" || str == "xlm" {
        return Ok((xdr::Asset::Native, str.to_string()));
    }
    let split: Vec<&str> = str.splitn(2, ':').collect();
    assert!(split.len() == 2, "invalid asset \"{str}\"");
    let code = split[0];
    let iss  = split[1];

    let issuer: xdr::AccountId = xdr::AccountId::from_str(iss)?;
    let re = regex::Regex::new("^[[:alnum:]]{1,12}$").expect("regex failed");
    assert!(re.is_match(code), "invalid asset \"{str}\"");
    let asset_code =  match code.len() {
        4 => xdr::AssetCode::CreditAlphanum4(xdr::AssetCode4(code.as_bytes().try_into()?)),
        12 => xdr::AssetCode::CreditAlphanum12(xdr::AssetCode12(code.as_bytes().try_into()?)),
        _ => panic!("invalid asset code length"),
    };
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
