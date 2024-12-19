use darling::FromDeriveInput;
use proc_macro::{self, TokenStream};
use quote::quote;
use syn::{parse_macro_input, DataStruct, DeriveInput};

// https://github.com/imbolc/rust-derive-macro-guide
#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(rpc_message))]
struct Opts {
    namespace: String,
}

#[proc_macro_derive(RpcMessageParams, attributes(rpc_message))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let opts = Opts::from_derive_input(&input).expect("Wrong options");
    let DeriveInput { ident, data, .. } = input;
    let method_name = match data {
        syn::Data::Struct(DataStruct { .. }) => {
            let name = stringify!(#ident);
            let name_no_params = name.strip_suffix("Params").unwrap_or(name);
            let first_char = name_no_params
                .chars()
                .next()
                .unwrap()
                .to_owned()
                .to_lowercase();
            let method = format!("{first_char}{}", &name_no_params[1..]);

            quote! {
                fn method()-> &'static str {
                    #method
                }
            }
        }
        _ => {
            panic!("cannot derive this on anything but a struct")
        }
    };
    let ns = opts.namespace;
    let namespace = quote! {
        fn namespace() -> Namespace {
         Namespace::try_from(#ns).unwrap()

        }
    };

    let output = quote! {
        impl RpcMessageParams for #ident {
            #method_name
            #namespace
        }
    };

    output.into()
}

