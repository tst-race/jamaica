use proc_macro::TokenStream;

mod code_gen;

#[proc_macro]
pub fn motif_code_generation(_input: TokenStream) -> TokenStream {
    static JSON_STR: &str = include_str!(env!("PROTO_SPEC_FILE"));
    code_gen::code_gen_impl(JSON_STR).into()
}
