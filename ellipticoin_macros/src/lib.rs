#[macro_use]
extern crate quote;
extern crate heck;
extern crate proc_macro;
extern crate syn;

use heck::CamelCase;
use proc_macro::TokenStream;
use proc_macro2::Span;
use syn::{
    braced, parenthesized, parse,
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    token, FnArg, Ident, ItemEnum, ItemFn, Result, Token, Type,
};

struct Block {
    ident: Ident,
    _brace_token: token::Brace,
    accessor_args: AccessorArgs,
}

impl Parse for Block {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        Ok(Block {
            ident: input.parse()?,
            _brace_token: braced!(content in input),
            accessor_args: content.parse()?,
        })
    }
}

struct AccessorArgs {
    attrs: Punctuated<Attribute, Token![;]>,
}

impl Parse for AccessorArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(AccessorArgs {
            attrs: input.parse_terminated(Attribute::parse)?,
        })
    }
}

#[derive(Clone)]
struct Attribute {
    ident: Ident,
    paren_token: token::Paren,
    inputs: Punctuated<FnArg, Token![,]>,
    r_arrow_token: Token![->],
    output: Type,
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
pub fn db_accessors(item: TokenStream) -> TokenStream {
    let Block {
        ident,
        accessor_args,
        ..
    } = parse(item).unwrap();
    let AccessorArgs { attrs, .. } = accessor_args;
    let attrs_vec = attrs.iter().cloned().collect::<Vec<Attribute>>();
    let base_namespace = Ident::new("Namespace", Span::call_site());
    let getters = attrs_vec
        .iter()
        .map(|attr| getter(&base_namespace, attr))
        .collect::<Vec<ItemFn>>();
    let setters = attrs_vec
        .iter()
        .map(|attr| setter(&base_namespace, attr))
        .collect::<Vec<ItemFn>>();
    let namespace: ItemEnum = namespace(base_namespace, &attrs_vec);
    return quote!(
        #namespace
        impl #ident {
            #(#getters)*
            #(#setters)*
        }
    )
    .into();
}

fn getter(base_namespace: &Ident, attr: &Attribute) -> ItemFn {
    let Attribute {
        ident,
        inputs,
        output,
        ..
    } = attr;
    let fn_ident = Ident::new(&format!("get_{}", &ident), Span::call_site());
    let mut new_inputs = inputs.clone();
    new_inputs.insert(0, parse_quote!(db: &mut ellipticoin_types::db::Db<B>));
    let namespace = Ident::new(&ident.to_string().to_camel_case(), Span::call_site());

    let key_parts = inputs.iter().map(|input| {
        if let syn::FnArg::Typed(syn::PatType { pat, .. }) = input {
            pat
        } else {
            panic!("Invalid type: {}", quote!(input))
        }
    });
    parse_quote!(
        pub fn #fn_ident<B: ellipticoin_types::db::Backend>(#new_inputs) -> #output {
            Self::get(
                db,
                [
                    u16::to_le_bytes(#base_namespace::#namespace as u16).to_vec(),
                    #(ellipticoin_types::traits::ToKey::to_key(&#key_parts), )*
                ].concat()
            )
        }
    )
}

fn setter(base_namespace: &Ident, attr: &Attribute) -> ItemFn {
    let Attribute {
        ident,
        inputs,
        output,
        ..
    } = attr;
    let fn_ident = Ident::new(&format!("set_{}", &ident), Span::call_site());
    let mut new_inputs = inputs.clone();
    new_inputs.insert(0, parse_quote!(db: &mut ellipticoin_types::Db<B>));
    new_inputs.push(parse_quote!(value: #output));
    let namespace = Ident::new(&ident.to_string().to_camel_case(), Span::call_site());
    let key_parts = inputs.iter().map(|input| {
        if let syn::FnArg::Typed(syn::PatType { pat, .. }) = input {
            pat
        } else {
            panic!("Invalid type: {}", quote!(input))
        }
    });
    parse_quote!(
    pub fn #fn_ident<B: ellipticoin_types::db::Backend>(#new_inputs) {
        Self::insert(
            db,
            [
                u16::to_le_bytes(#base_namespace::#namespace as u16).to_vec(),
                #(ellipticoin_types::traits::ToKey::to_key(&#key_parts), )*
            ].concat(),
            value
        );
    })
}

fn namespace(base_namespace: Ident, attrs: &Vec<Attribute>) -> ItemEnum {
    let variants: Vec<Ident> = attrs
        .iter()
        .map(|Attribute { ident, .. }| {
            Ident::new(&ident.to_string().to_camel_case(), Span::call_site())
        })
        .collect();
    parse_quote!(
        #[repr(u16)]
        pub enum #base_namespace {
            #(#variants),*
        }
    )
}
