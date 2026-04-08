use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_derive(EnumParam)]
pub fn derive_enum_param(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = input.ident;

    // Ensure this is only applied to enums
    let Data::Enum(data_enum) = input.data else {
        panic!("EnumParam can only be derived for enums");
    };

    // Extract all variants
    let variants: Vec<_> = data_enum.variants.into_iter().collect();
    
    // Extract variant identifiers and strings for use in generated code
    let variant_idents: Vec<_> = variants.iter().map(|v| v.ident.clone()).collect();
    let variant_strings: Vec<_> = variants.iter().map(|v| v.ident.to_string()).collect();

    // Intelligently find the default variant
    // Fallback to the first variant if #[default] is missing
    let mut default_variant = variants.first().expect("Enum must have at least one variant").ident.clone();
    for variant in &variants {
        // Look for the #[default] attribute
        if variant.attrs.iter().any(|attr| attr.path().is_ident("default")) {
            default_variant = variant.ident.clone();
            break;
        }
    }

    // Generate the Rust code to implement your trait
    let expanded = quote! {
        impl ::karbeat_plugin_types::parameter::EnumParam for #name {
            #[inline(always)]
            fn to_index(self) -> usize {
                self as usize
            }

            #[inline(always)]
            fn from_index(index: usize) -> Self {
                // Generate an array of all variants
                let variants = [ #( #name::#variant_idents ),* ];
                
                if index < variants.len() {
                    variants[index]
                } else {
                    // Fallback to the detected default variant
                    #name::#default_variant
                }
            }

            fn variants() -> &'static [&'static str] {
                // Generate the array of string slices
                &[ #( #variant_strings ),* ]
            }
        }
    };

    // Hand the generated code back to the compiler
    TokenStream::from(expanded)
}