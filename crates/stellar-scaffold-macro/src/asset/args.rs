use std::fmt::Display;

use syn::parse::{Parse, ParseStream};
use syn::{Ident, LitInt, Token};

// Represents the full asset macro arguments.
#[derive(Debug)]
pub struct AssetArgs {
    pub asset: AssetSpec,
    pub decimals: u32,
}

impl Parse for AssetArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let asset: AssetSpec = input.parse()?;
        let decimals = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            input.parse::<LitInt>()?.base10_parse()?
        } else {
            7
        };
        Ok(AssetArgs { asset, decimals })
    }
}

impl From<proc_macro2::TokenStream> for AssetArgs {
    fn from(value: proc_macro2::TokenStream) -> Self {
        syn::parse2::<AssetArgs>(value).expect("failed to parse asset! args")
    }
}

// The `asset` piece.
#[derive(Debug)]
pub enum AssetSpec {
    Native(Ident),
    Asset(Ident, Ident),
}

impl Display for AssetSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AssetSpec::Native(ident) => write!(f, "{ident}"),
            AssetSpec::Asset(code, asset) => write!(f, "{code}:{asset}"),
        }
    }
}

impl Parse for AssetSpec {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let symbol: Ident = input.parse()?;
        Ok(if input.peek(Token![:]) {
            input.parse::<Token![:]>()?;
            AssetSpec::Asset(symbol, input.parse()?)
        } else {
            AssetSpec::Native(symbol)
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quote::quote;
    #[test]
    fn parse_native() {
        let _: AssetArgs = quote! { native }.into();
    }
    #[test]
    fn parse_asset() {
        let _: AssetArgs =
            quote! { USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN, 8 }.into();
    }
}
