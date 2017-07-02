extern crate proc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{Body, VariantData, MetaItem, NestedMetaItem, Lit, Ident};
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
            let mut handler: Option<Ident> = None;

            for attr in &ast.attrs {
                match attr.value {
                    MetaItem::List(ref ident, ref items) => {
                        for item in items {
                            match item {
                                &NestedMetaItem::MetaItem(MetaItem::NameValue(ref ident, Lit::Str(ref value, _))) => {
                                    match ident.as_ref() {
                                        "path" => path = Some(value.as_str()),
                                        "handler" => handler = Some(value.as_str().into()),
                                        _ => panic!("unsupported attribute item"),
                                    }
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
            let handler = handler.expect("failed to parse attribute");

            // extract route parameters
            let params: Vec<&str> = path.split("/")
                .filter_map(|s| if s.starts_with(":") {
                    Some(s.trim_left_matches(":"))
                } else {
                    None
                })
                .collect();

            let handler_prefix_code = if params.len() == 1 {
                let param = &params[0];
                let ident = Ident::new(*param);
                quote! {
                    let #ident = {
                        let router = req.extensions.get::<::router::Router>().unwrap();
                        let #ident = router.find(#param).unwrap().parse().map_err(|err| ::iron::IronError::new(err, ::iron::status::InternalServerError))?;
                        #ident
                    };
                }
            } else if params.len() > 0 {
                let param_bodies = params.iter().map(|&param| {
                    let ident = Ident::new(param);
                    quote!{
                        let #ident = router.find(#param).unwrap().parse().map_err(|err| ::iron::IronError::new(err, ::iron::status::InternalServerError))?;
                    }
                });
                let param_names: Vec<_> = params
                    .iter()
                    .map(|&param| {
                        let ident = Ident::new(param);
                        quote!(#ident)
                    })
                    .collect();
                let param_names2 = param_names.clone();
                quote! {
                    let (#(#param_names),*) = {
                        let router = req.extensions.get::<::router::Router>().unwrap();
                        #(#param_bodies)*
                        (#(#param_names2),*)
                    };
                }
            } else {
                quote!()
            };

            let handler_params = if params.len() > 0 {
                let params = params.iter().map(|&param| {
                    let ident = Ident::new(param);
                    quote!(#ident,)
                });
                quote!((req, #(#params)*))
            } else {
                quote!((req))
            };

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
                impl ::iron::Handler for #name {
                    #[inline]
                    fn handle(&self, req: &mut ::iron::Request) -> ::iron::IronResult<::iron::Response> {
                        #handler_prefix_code
                        #handler #handler_params
                    }
                }
            )
        }
        _ => panic!("implementation of Route is only supported for unit struct"),
    };

    tokens.parse().unwrap()
}
