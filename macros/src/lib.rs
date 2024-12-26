use core::panic;

use darling::FromDeriveInput;
use proc_macro::{self, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, TypePath};

// https://github.com/imbolc/rust-derive-macro-guide
#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(some_attr))]
struct Opts {}

#[proc_macro_derive(SomeTrait, attributes(rpc_request))]
pub fn derive(input: TokenStream) -> TokenStream {
    unimplemented!()
}
