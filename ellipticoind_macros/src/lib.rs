#[macro_use]
extern crate quote;
extern crate heck;
extern crate proc_macro;
extern crate syn;

use heck::CamelCase;
use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::FnArg;

use syn::{
    parenthesized, parse,
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    token, Ident, ItemEnum, Result, Token, Type, Variant,
};

#[derive(Debug)]
struct AccessorArgs {
    attrs: Punctuated<Attribute, Token![;]>,
}

#[derive(Debug, Clone)]
struct Attribute {
    ident: Ident,
    paren_token: token::Paren,
    inputs: Punctuated<FnArg, Token![,]>,
    r_arrow_token: Token![->],
    output: Type,
}

impl Parse for AccessorArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(AccessorArgs {
            attrs: input.parse_terminated(Attribute::parse)?,
        })
    }
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Attribute {
            ident: input.parse()?,
            paren_token: parenthesized!(content in input),
            inputs: content.parse_terminated(FnArg::parse).expect("expected fn"),
            r_arrow_token: input.parse()?,
            output: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn state_accessors(item: TokenStream) -> TokenStream {
    let AccessorArgs { attrs, .. } = parse(item).unwrap();

    let attrs2: Vec<(
        Ident,
        syn::punctuated::Punctuated<syn::FnArg, syn::token::Comma>,
        Type,
    )> = attrs
        .iter()
        .cloned()
        .map(
            |Attribute {
                 ident,
                 output,
                 inputs,
                 ..
             }| { (ident, inputs, output) },
        )
        .collect();
    let get_fn = Ident::new(&"get_state", Span::call_site());
    let base_namespace = Ident::new("Namespace", Span::call_site());
    let getters = attrs2
        .iter()
        .map(|(ident, inputs, ty)| {
            let getter_name = Ident::new(&format!("get_{}", &ident), Span::call_site());
            let namespace = Ident::new(&ident.to_string().to_camel_case(), Span::call_site());
            let mut p: Punctuated<syn::Expr, syn::token::Comma> = Punctuated::new();
            for input in inputs {
                if let syn::FnArg::Typed(syn::PatType { pat, .. }) = input {
                    p.push(parse_quote!(#pat.into()))
                } else {
                    panic!("oh no")
                }
            }
            let mut inputs3 = inputs.clone();
            let fn_arg: FnArg = parse_quote!(api: &mut API);
            inputs3.insert(0, fn_arg);
            parse_quote!(
            pub fn #getter_name<API: ellipticoin::API>(#inputs3) -> #ty {
                api.#get_fn(
                    CONTRACT_NAME,
                    [
                        [#base_namespace::#namespace as u8].to_vec(),
                        #p
                    ]
                    .concat(),
                )
                .unwrap_or(Default::default())
            })
        })
        .collect::<Vec<syn::ItemFn>>();
    let set_fn = Ident::new(&"set_state", Span::call_site());
    let setters = attrs2
        .iter()
        .map(|(ident, inputs, ty)| {
            let setter_name = Ident::new(&format!("set_{}", &ident), Span::call_site());
            let namespace = Ident::new(&ident.to_string().to_camel_case(), Span::call_site());
            let mut p: Punctuated<syn::Expr, syn::token::Comma> = Punctuated::new();
            for input in inputs {
                if let syn::FnArg::Typed(syn::PatType { pat, .. }) = input {
                    p.push(parse_quote!(#pat.into()))
                } else {
                    panic!("oh no")
                }
            }
            let fn_arg: FnArg = parse_quote!(api: &mut API);
            let mut inputs3 = inputs.clone();
            inputs3.insert(0, fn_arg);
            parse_quote!(
                        pub fn #setter_name<API: ellipticoin::API>(#inputs3, value: #ty) {
                     api.#set_fn(
                            CONTRACT_NAME,
            [[#base_namespace::#namespace as u8].to_vec(),
                                    #p
            ].concat(), value);
                        })
        })
        .collect::<Vec<syn::ItemFn>>();
    let mut namespace: ItemEnum = parse_quote!(
        pub enum #base_namespace {}
    );
    for (ident, _inputs, _ty) in attrs2.iter() {
        let varient = Variant {
            attrs: vec![],
            ident: Ident::new(&ident.to_string().to_camel_case(), Span::call_site()),
            fields: syn::Fields::Unit,
            discriminant: None,
        };
        namespace.variants.push(varient);
    }
    (quote! {
        #namespace
        #(#getters)*
        #(#setters)*
    })
    .into()
}
