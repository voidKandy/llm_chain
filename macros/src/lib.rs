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
    match data {
        syn::Data::Struct(DataStruct { fields, .. }) => {
            let name = stringify!(#ident);
            let name_no_params = name.strip_suffix("Params").unwrap_or(name);
            let first_char = name_no_params
                .chars()
                .next()
                .unwrap()
                .to_owned()
                .to_lowercase();
            let method = format!("{first_char}{}", &name_no_params[1..]);

            let mut from_json_body = quote! {};
            let mut create_self_body = quote! {};

            for f in fields {
                let id = f.ident.unwrap();
                from_json_body = quote! {
                    #from_json_body
                    let #id = serde_json::from_value(json.get("#id")
                        .expect("field '#id' does not exist").to_owned())
                        .expect("field '#id' does not implement deserialize");
                };

                create_self_body = quote! {
                    #create_self_body
                    #id,
                }
            }

            let create_self = quote! {
                Ok(Self {
                    #create_self_body
                })
            };

            let from_json = quote! {
            fn try_from_json(json: &serde_json::Value) -> MainResult<Self>
                  {
                        #from_json_body
                        #create_self
                    }


            };

            let method_name = quote! {
                fn method()-> &'static str {
                    #method
                }
            };

            let ns = opts.namespace;
            let namespace = quote! {
                fn namespace() -> Namespace {
                 Namespace::try_from(#ns).unwrap()

                }
            };

            // let rpc_message

            let output = quote! {
                impl RpcMessageParams for #ident {
                    #method_name
                    #namespace
                    #from_json
                }
            };

            output.into()
        }
        _ => {
            panic!("cannot derive this on anything but a struct")
        }
    }
}

