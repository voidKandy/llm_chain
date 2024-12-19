use darling::FromDeriveInput;
use proc_macro::{self, TokenStream};
use quote::{format_ident, quote};
use syn::{parse_macro_input, DataStruct, DeriveInput};

// https://github.com/imbolc/rust-derive-macro-guide
#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(rpc_request))]
struct Opts {
    namespace: String,
    response: Option<String>,
}

#[proc_macro_derive(RpcRequest, attributes(rpc_request))]
pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input);
    let opts = Opts::from_derive_input(&input).expect("Wrong options");
    let DeriveInput { ident, data, .. } = input;
    match data {
        syn::Data::Struct(DataStruct { fields, .. }) => {
            let name = format!("{ident}");
            let name_no_suffix = name
                .strip_suffix("Request")
                .expect("make sure to put 'Request' at the end of your struct name");
            let struct_name = format_ident!("{}", name_no_suffix);
            let response_struct_name = match opts.response {
                Some(res) => format_ident!("{}", res),
                None => format_ident!("{}Response", name_no_suffix),
            };
            let first_char = name_no_suffix
                .chars()
                .next()
                .unwrap()
                .to_owned()
                .to_lowercase();
            let method = format!("{first_char}{}", &name_no_suffix[1..]);

            let mut from_json_body = quote! {};
            let mut create_self_body = quote! {};

            for f in fields {
                let id = f.ident.unwrap();
                let json_name = format_ident!("{}_json", id);
                let id_string = format!("{id}");
                let not_exist = format!("field '{id_string}' does not exist");
                let not_deserialize = format!("field '{id_string}' does not implement deserialize");
                from_json_body = quote! {
                    #from_json_body
                    let #json_name =  json.get(#id_string).ok_or(#not_exist)?.to_owned();
                    let #id = serde_json::from_value(#json_name).map_err(|_|#not_deserialize)?;
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
              fn try_from_json(json: &serde_json::Value) -> MainResult<Self> {
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

            let output = quote! {
                impl RpcResponse for #response_struct_name {}
                impl RpcRequest for #ident {
                    type Response = #response_struct_name;
                    #from_json
                    #method_name
                    #namespace
                }
            };

            output.into()
        }
        _ => {
            panic!("cannot derive this on anything but a struct")
        }
    }
}
