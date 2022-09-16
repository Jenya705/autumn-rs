use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn attr(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("{:?} {:?}", attr, item);
    attr
}