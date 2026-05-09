//! Proc macros for the PDF module workspace.
//!
//! Provides derive macros to reduce boilerplate:
//! - `#[derive(Builder)]`: auto-generates builder-pattern methods for structs

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

/// Derive macro that generates builder-pattern `with_*` methods
/// for each named field on a struct.
///
/// - `Option<T>` fields get a `with_<field>(value: T)` method that wraps in `Some`.
/// - Non-optional fields get a `with_<field>(value: impl Into<T>)` setter.
/// - `bool` fields also get a `with_<field>()` method that sets to `true`.
///
/// # Example
///
/// ```ignore
/// use pdf_macros::Builder;
///
/// #[derive(Builder)]
/// pub struct ToolContext {
///     pub execution_id: String,
///     pub org_id: Option<String>,
///     pub enable_cache: bool,
/// }
///
/// let ctx = ToolContext::new("exec-1")
///     .with_org_id("org-42")
///     .with_enable_cache();
/// ```
#[proc_macro_derive(Builder)]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    name,
                    "Builder can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(name, "Builder can only be derived for structs")
                .to_compile_error()
                .into();
        }
    };

    let mut builder_methods = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let method_name = syn::Ident::new(&format!("with_{}", field_name), field_name.span());
        let field_type = &field.ty;

        // Check if the field type is Option<T>
        if is_option(field_type) {
            let inner_type = extract_option_inner(field_type);
            builder_methods.push(quote! {
                pub fn #method_name(mut self, value: #inner_type) -> Self {
                    self.#field_name = Some(value);
                    self
                }
            });
        } else if is_bool(field_type) {
            builder_methods.push(quote! {
                pub fn #method_name(mut self) -> Self {
                    self.#field_name = true;
                    self
                }
            });
        } else {
            builder_methods.push(quote! {
                pub fn #method_name(mut self, value: impl Into<#field_type>) -> Self {
                    self.#field_name = value.into();
                    self
                }
            });
        }
    }

    let expanded = quote! {
        impl #name {
            #(#builder_methods)*
        }
    };

    TokenStream::from(expanded)
}

/// Check if a type is `Option<T>`.
fn is_option(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

/// Extract the inner type `T` from `Option<T>`.
fn extract_option_inner(ty: &syn::Type) -> &syn::Type {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return inner;
                }
            }
        }
    }
    ty
}

/// Check if a type is `bool`.
fn is_bool(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "bool";
        }
    }
    false
}
