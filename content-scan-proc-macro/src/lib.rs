mod derive;
use proc_macro::*;
extern crate proc_macro;

#[proc_macro_derive(ContentType)]
pub fn derive_enum_var_map(input: TokenStream) -> TokenStream {
    match derive::process_content_type(input) {
        Ok(ts) => ts,
        Err(msg) => format!("compile_error!({:?});", msg).parse().unwrap(),
    }
}
