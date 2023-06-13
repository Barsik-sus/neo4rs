use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::{DeriveInput, MetaList};

fn is_option(ty: &syn::Type) -> bool {
    if let syn::Type::Path(syn::TypePath { qself: None, path }) = ty {
        let segments_str = &path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join(":");
        ["Option", "std:option:Option", "core:option:Option"]
            .iter()
            .find(|s| segments_str == *s)
            .is_some()
    } else {
        false
    }
}

#[proc_macro_derive(FromBoltType)]
pub fn from_bolt_type(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let struct_name = &ast.ident;

    let fields = if let syn::Data::Struct(structure) = ast.data {
        match structure.fields {
            syn::Fields::Named(syn::FieldsNamed { named, .. }) => named,
            syn::Fields::Unnamed(_) => {
                unimplemented!(concat!(stringify!(#name), ": unnamed fields not supported"))
            }
            syn::Fields::Unit => syn::punctuated::Punctuated::new(),
        }
    } else {
        unimplemented!(concat!(stringify!(#name), ": not a struct"));
    };

    let from_map_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

        if is_option(ty) {
            quote! {
                #name: value.get(stringify!(#name))
            }
        } else {
            quote! {
                #name: value.get(stringify!(#name)).unwrap()
            }
        }
    });

    let expanded = quote!(
        impl From<BoltMap> for #struct_name {
            fn from(value: BoltMap) -> Self {
                Self {
                    #(#from_map_fields,)*
                }
            }
        }

        impl From<BoltNode> for #struct_name {
            fn from(value: BoltNode) -> Self {
                let value = value.properties;
                
                value.into()
            }
        }

        impl From<BoltType> for #struct_name {
            fn from(value: BoltType) -> Self {
                match value {
                    BoltType::Map(inner) => inner.into(),
                    BoltType::Node(inner) => inner.into(),
                    v => panic!("{}: can not be made from {v:?}", stringify!(#struct_name))
                }
            }
        }
    );

    expanded.into()
}

#[proc_macro_derive(BoltStruct, attributes(signature))]
pub fn derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let struct_name = &ast.ident;

    let meta = ast.attrs.get(0).unwrap().parse_meta().unwrap();

    let values: Vec<syn::LitInt> = match meta {
        syn::Meta::List(MetaList { nested, .. }) => {
            nested.into_iter().map(|nested_meta| match nested_meta {
                syn::NestedMeta::Lit(syn::Lit::Int(value)) => value,
                _ => panic!(concat!(
                    stringify!(#struct_name),
                    ": signature is not literal"
                )),
            })
        }
        _ => panic!(concat!(stringify!(#struct_name), ": invalid signature")),
    }
    .collect();

    let (struct_marker, struct_signature) = if values.len() == 2 {
        let marker = values.get(0).unwrap();
        let sig = values.get(1).unwrap();
        (quote! { #marker}, quote! {Some(#sig)})
    } else {
        let marker = values.get(0).unwrap();
        (quote! { #marker}, quote! { None::<u8> })
    };

    let fields = if let syn::Data::Struct(structure) = ast.data {
        match structure.fields {
            syn::Fields::Named(syn::FieldsNamed { named, .. }) => named,
            syn::Fields::Unnamed(_) => {
                unimplemented!(concat!(stringify!(#name), ": unnamed fields not supported"))
            }
            syn::Fields::Unit => syn::punctuated::Punctuated::new(),
        }
    } else {
        unimplemented!(concat!(stringify!(#name), ": not a struct"));
    };

    let serialize_fields = fields.iter().map(|f| {
        let name = &f.ident;
        quote! {
            let #name: bytes::Bytes = self.#name.into_bytes(version)?
        }
    });

    let allocate_bytes = fields.iter().map(|f| {
        let name = &f.ident;
        quote! {
            total_bytes += #name.len()
        }
    });

    let put_bytes = fields.iter().map(|f| {
        let name = &f.ident;
        quote! {
            bytes.put(#name)
        }
    });

    let deserialize_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let typ = &f.ty;
        quote! {
            #name: #typ::parse(version, input.clone())?
        }
    });

    let expanded = quote! {
        use std::convert::*;
        use bytes::*;

        impl #struct_name {

            pub fn into_bytes(self, version: crate::version::Version) -> crate::errors::Result<bytes::Bytes> {
                #(#serialize_fields;)*
                let mut total_bytes = std::mem::size_of::<u8>() + std::mem::size_of::<u8>();
                #(#allocate_bytes;)*
                let mut bytes = BytesMut::with_capacity(total_bytes);
                bytes.put_u8(#struct_marker);
                if let Some(signature) = #struct_signature {
                    bytes.put_u8(signature);
                }
                #(#put_bytes;)*
                Ok(bytes.freeze())
            }

        }

        impl #struct_name {
            pub fn can_parse(version: crate::version::Version, input: std::rc::Rc<std::cell::RefCell<bytes::Bytes>>) -> bool {
                match (#struct_marker, #struct_signature) {
                    (marker, Some(signature)) =>  {
                        input.borrow().len() >= 2 && input.borrow()[0] == marker && input.borrow()[1] == signature
                    },
                    (marker, None) => {
                        input.borrow().len() >= 1 && input.borrow()[0] == marker
                    }
                    _ => false
                }
            }
        }

        impl #struct_name {

            pub fn parse(version: crate::version::Version, input: std::rc::Rc<std::cell::RefCell<bytes::Bytes>>) -> crate::errors::Result<#struct_name> {

                match (#struct_marker, #struct_signature) {
                    (_, Some(_)) =>  {
                        input.borrow_mut().get_u8();
                        input.borrow_mut().get_u8();
                    },
                    (_, None) => {
                        input.borrow_mut().get_u8();
                    }
                }

                Ok(#struct_name {
                    #(#deserialize_fields,)*
                })
            }
        }


    };
    expanded.into()
}
