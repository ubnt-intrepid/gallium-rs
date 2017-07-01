extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{Body, VariantData, MetaItem, NestedMetaItem, Lit};
use quote::Tokens;

#[proc_macro_derive(Route, attributes(get, post, put, delete, option))]
pub fn derive_iron_route(input: TokenStream) -> TokenStream {
    let s = input.to_string();
    let ast = syn::parse_derive_input(&s).unwrap();

    let tokens = match ast.body {
        Body::Struct(VariantData::Unit) => {
            let name = &ast.ident;
            let route_id = name.to_string();

            let mut method: Option<Tokens> = None;
            let mut path: Option<&str> = None;

            for attr in &ast.attrs {
                match attr.value {
                    MetaItem::List(ref ident, ref items) => {
                        for item in items {
                            match item {
                                &NestedMetaItem::MetaItem(MetaItem::NameValue(ref ident, Lit::Str(ref value, _)))
                                    if ident.as_ref() == "path" => {
                                    path = Some(value.as_str());
                                }
                                _ => panic!(""),
                            }
                        }
                        match ident.as_ref() {
                            "get" => method = Some(quote!(Get)),
                            "post" => method = Some(quote!(Post)),
                            "put" => method = Some(quote!(Put)),
                            "delete" => method = Some(quote!(Delete)),
                            "option" => method = Some(quote!(Option)),
                            _ => panic!("unsupported HTTP method"),
                        }
                    }
                    _ => panic!(""),
                }
            }
            let method = method.expect("failed to parse attribute");
            let path = path.expect("failed to parse attribute");

            quote!(
                impl ::iron_router_ext::Route for #name {
                    fn route_id() -> &'static str {
                        #route_id
                    }
                    fn route_method() -> ::iron::method::Method {
                        ::iron::method::Method:: #method
                    }
                    fn route_path() -> &'static str {
                        #path
                    }
                }
            )
        }
        _ => panic!("implementation of Route is only supported for unit struct"),
    };

    tokens.parse().unwrap()
}
