use std::fmt::Display;

use darling::FromMeta;
use darling::ast::NestedMeta;
use quote::ToTokens as _;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Meta, Token};

// Represents the full asset macro arguments.
#[derive(Debug)]
pub struct AssetArgs {
    pub asset: AssetSpec,
    pub decimals: u32,
}

impl From<proc_macro2::TokenStream> for AssetArgs {
    fn from(value: proc_macro2::TokenStream) -> Self {
        // Turn the comma‑separated list into `NestedMeta` items.
        let metas: Vec<NestedMeta> =
            NestedMeta::parse_meta_list(value).expect("failed to parse asset! args");

        // First argument: AssetSpec
        let asset = AssetSpec::from_nested_meta(&metas[0]).expect("failed to parse asset spec");

        // Second argument: u32 (optional, defaults to 7 for native assets)
        let decimals = metas
            .get(1)
            .map(|m| u32::from_nested_meta(m).expect("failed to parse decimals"))
            .unwrap_or(7);
        AssetArgs { asset, decimals }
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

impl FromMeta for AssetSpec {
    fn from_string(value: &str) -> darling::Result<Self> {
        // This gets called for simple string literals like "USDC:G1333"
        syn::parse_str::<AssetSpec>(value).map_err(darling::Error::custom)
    }

    fn from_value(value: &syn::Lit) -> darling::Result<Self> {
        if let syn::Lit::Str(lit_str) = value {
            syn::parse_str::<AssetSpec>(&lit_str.value()).map_err(darling::Error::custom)
        } else {
            Err(darling::Error::custom(
                "expected string literal for AssetSpec",
            ))
        }
    }

    fn from_meta(meta: &Meta) -> darling::Result<Self> {
        // Handles bare tokens, e.g. USDC:G1333 without quotes
        let tokens = match meta {
            Meta::Path(path) => path.clone().into_token_stream(),
            Meta::NameValue(nv) => nv.value.clone().into_token_stream(),
            Meta::List(list) => list.tokens.clone(),
        };
        syn::parse2::<AssetSpec>(tokens).map_err(darling::Error::custom)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use quote::quote;
    #[test]
    fn parse() {
        let _: AssetArgs = quote! { native }.into();
    }
}
