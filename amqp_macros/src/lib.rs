use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;

#[proc_macro_derive(FieldCount)]
pub fn derive_field_count(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    let ast: DeriveInput = syn::parse(input).unwrap();

    let st = match ast.data {
        syn::Data::Struct(st) => st,
        syn::Data::Enum(_) => todo!(),
        syn::Data::Union(_) => todo!(),
    };
    let name = ast.ident;
    let count = st.fields.len();
    let expanded = quote! {
        impl #name {
            pub fn field_count() -> usize {
                #count
            }
        }
    };

    expanded.into()
}
