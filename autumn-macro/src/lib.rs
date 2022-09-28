mod module;

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn mod_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    TokenStream::new()
}