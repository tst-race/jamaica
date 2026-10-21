// Copyright 2024 Two Six Technologies.

///
/// Implementation of the code generator using proc_macro2.
///
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::{
    collections::{HashMap, HashSet},
    io::Write,
};
// use std::collections::hash_map::DefaultHasher;
use std::hash::{DefaultHasher, Hasher};
// use prettyplease;

pub fn code_gen_impl(input: &str) -> TokenStream {
    // eprintln!("code_gen_impl input = {:?}", input);

    let mut hasher = DefaultHasher::new();
    hasher.write(input.as_bytes());
    // trace!("{} with hash {}", log_message, hasher.finish());

    let json_value: serde_json::Value = match serde_json::from_str(input) {
        Ok(json_value) => json_value,
        Err(err) => panic!("failed to convert LitStr to JSON: {}", err),
    };

    // eprintln!("JSON = {}", json_value);

    let mut funcs = vec![];
    let mut supported_fields = HashSet::new();
    // The set of fields that can be used to embed client control messages.
    let mut embeddable_fields = HashSet::new();
    let mut field_name_to_func_name = HashMap::new();
    let mut match_arms = TokenStream::new();
    let mut match_arms_for_field_generator_func = TokenStream::new();
    if let Some(func_specs) = json_value.as_array() {
        for func_spec in func_specs {
            let operation = match func_spec["operation"].as_str() {
                Some(val) => val,
                None => panic!("Failed to get operation from JSON"),
            };
            // eprintln!("{}", operation);

            let field_name = match func_spec["output"].as_str() {
                Some(val) => val,
                None => panic!("Failed to get operation from JSON"),
            };
            // eprintln!("field_name = {}", field_name);

            supported_fields.insert(field_name.to_string());

            match func_spec["can_embed"].as_bool() {
                Some(val) => {
                    if val {
                        embeddable_fields.insert(field_name.to_string());
                    }
                }
                None => (),
            }

            let func_name = field_name.replace(".", "__");
            // eprintln!("func_name = {}", func_name);
            let func_name = syn::Ident::new(func_name.as_str(), Span::call_site());
            field_name_to_func_name.insert(field_name, func_name.clone());

            // Operations
            //      constant
            //      encrypted
            //      flag
            //      id
            //      length
            //      sequence
            //      value_if_field_is_present (same as flag?)
            match operation {
                "length" => {
                    let target_field_name = func_spec["args"]
                        .as_array()
                        .unwrap_or_else(|| panic!("value for args key should be an array"))[0]
                        .clone()
                        .as_str()
                        .unwrap_or_else(|| panic!("failed to get arg value from JSON"))
                        .to_string();
                    let func = quote! {
                        fn #func_name(param: Vec<u8>) -> usize {
                            param.len()
                        }
                    };
                    funcs.push(func);

                    match_arms_for_field_generator_func.extend(quote! {
                        #field_name => {
                            // let target_field = fields.iter().find(|x| x.field.as_ref().is_some() && x.field.as_ref().unwrap() == #target_field_name)
                            //         .unwrap_or_else(|| {
                            //             error!("failed to get target field {} for arg for length field {}. Defaulting to length 0.", #target_field_name, #field_name);
                            //             FieldJson {
                            //                 raw: None,
                            //                 field: None,
                            //                 size: None,
                            //                 arg: Some(0u64),
                            //             }
                            //         });
                            // // let result = target_field.arg.unwrap() / 8;

                            // let result = match target_field.arg {
                            //     Some(value) => value as usize,
                            //     None => {
                            //         panic!("failed to get arg for target field {}", #target_field_name);
                            //     }
                            // };

                            let target_field_size = match fields.iter().find(|x| x.field.as_ref().is_some() && x.field.as_ref().unwrap() == #target_field_name) {
                                Some(field) => match field.arg {
                                        Some(value) => value as usize,
                                        None => {
                                            panic!("failed to get arg for target field {}", #target_field_name);
                                        },
                                    }
                                    None => {
                                        log::error!("failed to get target field {} for arg for length field {}. Defaulting to length 0.", #target_field_name, #field_name);
                                        0usize
                                    }
                                
                            };
                                    // .unwrap_or_else(|| {
                                    //     error!("failed to get target field {} for arg for length field {}. Defaulting to length 0.", #target_field_name, #field_name);
                                    //     FieldJson {
                                    //         raw: None,
                                    //         field: None,
                                    //         size: None,
                                    //         arg: Some(0u64),
                                    //     }
                                    // });

                            // TODO: not actually calling the function here. Will field sizes always
                            // be known ahead of time in the JSON? If so, this is fine, if not then
                            // need to iron the field dependency structure.
                            // let result = #func_name();
                            let result = target_field_size.to_be_bytes().to_vec();
                            let size_in_bits = result.len() * 8;
                            return Ok((result, size_in_bits));
                        }
                    });
                }
                "value_if_field_is_present" => {
                    // let target_field_name = func_spec["args"]
                    //     .as_array()
                    //     .unwrap_or_else(|| panic!("value for args key should be an array"))[0]
                    //     .clone()
                    //     .as_str()
                    //     .unwrap_or_else(|| panic!("failed to get arg value from JSON"))
                    //     .to_string();
                    let value_to_return = func_spec["args"]
                        .as_array()
                        .unwrap_or_else(|| panic!("value for args key should be an array"))[1]
                        .clone()
                        .as_i64()
                        .unwrap_or_else(|| panic!("failed to get arg value from JSON"))
                        as u8;
                    let func = quote! {
                        fn #func_name(param: Vec<u8>) -> Option<u8> {
                            if !param.is_empty() {
                                return Some(#value_to_return);
                            }
                            None
                        }
                    };
                    funcs.push(func);

                    match_arms_for_field_generator_func.extend(quote! {
                        #field_name => {
                            // let does_target_field_exist = fields.iter().any(|x| x.field.as_ref().is_some() && x.field.as_ref().unwrap() == #target_field_name);

                            // // TODO: get the actual size.
                            // let size_in_bits = 8;

                            // // TODO: not actually calling the function here.
                            // if does_target_field_exist {
                            //     return Ok((#value_to_return, size_in_bits));
                            // } else {
                            //     // TODO: what should we do in the else case??????
                            //     return Ok((0, size_in_bits));
                            // }

                            return Ok((vec![], 0));
                        }
                    });
                }
                "constant" => {
                    let value_to_return = func_spec["returns"].as_i64().unwrap_or_else(|| {
                        panic!("failed to get return value for func type constant from JSON")
                    });
                    assert!(
                        value_to_return <= u32::MAX as i64,
                        "return value too large for constant operation"
                    );
                    let value_to_return = value_to_return as u32;
                    // TODO: what size should this be? isn't necessarily u8. Return a value and a size?c
                    let func = quote! {
                        fn #func_name() -> u32 {
                            #value_to_return
                        }
                    };
                    funcs.push(func);

                    match_arms_for_field_generator_func.extend(quote! {
                        #field_name => {
                            let result = #func_name();
                            let result = result.to_be_bytes().to_vec();
                            let size_in_bits = result.len() * 8;
                            return Ok((result, size_in_bits));
                        }
                    });
                }
                "encrypted" => {
                    let func = quote! {
                        fn #func_name(size: usize) -> Vec<u8> {
                            (0..size)
                                .map(|_| rand::random())
                                .collect()
                        }
                    };
                    funcs.push(func);

                    match_arms_for_field_generator_func.extend(quote! {
                        #field_name => {
                            let field_size_in_bits = match field.arg {
                                Some(value) => value as usize,
                                None => {
                                    panic!("failed to get arg for field {}", #field_name);
                                }
                            };
                            let result = #func_name(field_size_in_bits / 8);
                            let size_in_bits = result.len() * 8;
                            return Ok((result, size_in_bits));
                        }
                    });
                }
                "sequence" => {
                    // TODO: make this good and customizable
                    // TODO: how do we pass in the previous value? Need to design the high level
                    // packet generator.
                    let func = quote! {
                        fn #func_name<T: std::ops::Add<i32, Output = T>>(previous_value: T) -> T {
                            previous_value + 1 // TODO: probably won't be 1
                            // TODO: handle overflow
                        }
                    };
                    funcs.push(func);

                    match_arms_for_field_generator_func.extend(quote! {
                        #field_name => {
                            // let result = #func_name();
                            // let result = result.to_be_bytes().to_vec();
                            // let size_in_bits = result.len() * 8;
                            // TODO: implement real logic!!!!!
                            // TODO: have a seed value? or random?
                            static SEQUENCE_COUNTER: AtomicU16 = AtomicU16::new(0);
                            Ok((SEQUENCE_COUNTER.fetch_add(1, Ordering::Relaxed).to_be_bytes().to_vec(), 16))
                        }
                    });
                }
                "id" => {
                    // TODO: how do we pass in the previous value? Need to design the high level
                    // packet generator.
                    let func = quote! {
                        fn #func_name(size: usize, previous_value: Vec<u8>) -> Vec<u8> {
                            if previous_value.is_empty() {
                                return (0..size)
                                    .map(|_| rand::random())
                                    .collect();
                            } else {
                                return previous_value;
                            }
                        }
                    };
                    funcs.push(func);

                    // let size_of_field = func_spec["args"]
                    //     .as_array()
                    //     .unwrap_or_else(|| panic!("value for args key should be an array"))[1]
                    //     .clone()
                    //     .as_i64()
                    //     .unwrap_or_else(|| panic!("failed to get arg value from JSON"))
                    //     as usize;

                    match_arms_for_field_generator_func.extend(quote! {
                        #field_name => {
                            // TODO: pass in a real previous field value
                            let field_size_in_bits = match field.arg {
                                Some(value) => value as usize,
                                None => {
                                    panic!("failed to get arg for field {}", #field_name);
                                }
                            };
                            assert!(field_size_in_bits % 8 == 0, "invalid field size {} for field {}", field_size_in_bits, #field_name);
                            let result = #func_name(field_size_in_bits / 8, vec![]);
                            return Ok((result, field_size_in_bits));
                        }
                    });
                }
                _ => {
                    panic!("invalid operation: {}", operation);
                    // compile_error!("invalid operation"); // TODO: possible to output the specific invalid operation?
                }
            }
        }
    }

    if funcs.is_empty() {
        panic!("no func specs found in proto spec file");
    }

    let supported_fields: Vec<String> = supported_fields.into_iter().collect();

    funcs.push(quote! {
        pub fn is_proto_field_supported(field_name: &str) -> bool {
            static SUPPORTED_FIELDS: &'static [&str] = &[#(#supported_fields),*];
            SUPPORTED_FIELDS.contains(&field_name)
        }
    });

    let embeddable_fields: Vec<String> = embeddable_fields.into_iter().collect();

    funcs.push(quote! {
        pub fn can_embed_control_message_in_field(field_name: &str) -> bool {
            static EMBEDDABLE_FIELDS: &'static [&str] = &[#(#embeddable_fields),*];
            EMBEDDABLE_FIELDS.contains(&field_name)
        }
    });

    // match_arms.extend(quote! {
    //     // ("quic.payload", $a:expr)=> { quic__payload($a) };
    //     // ("quic.payload")=> { quic__payload(field.size.unwrap()) };
    //     ("quic.payload", $a:expr)=> { quic__payload($a.arg.unwrap() as usize) };

    // });
    // match_arms.extend(quote! {
    //     ("quic.version")=> { quic__version() };
    // });

    // funcs.push(quote! {
    //     macro_rules! generate_field {
    //         ($func_name:ident, $( $args:expr ),* ) => {
    //             $func_name($( $args ),*)
    //         };
    //     }
    // });
    // let mut match_arms = TokenStream::new();
    // for (key, func_name) in field_name_to_func_name.iter() {
    //     match_arms.extend(quote! {
    //         #key => {
    //             // println!("match arm!!! {:?}", #func_name)
    //             // #func_name($( $args ),*);
    //             #func_name($($args),*);
    //             // #func_name(); // WORKING sort of, just doesn't take any args lmaoooooo
    //             // println!("{:?}", func_name.to_string());
    //         }
    //         //     // ($field_name:expr) => {
    //         //         // $func_name($( $args ),*)
    //         //         println!("{}", #key);
    //         //     // };
    //         // }
    //     });
    // }

    funcs.push(quote! {
        macro_rules! generate_field {
            #match_arms
            ($func_name:expr, $($arg:expr),*) => {
                compile_error!("Unknown function or incorrect number of arguments")
            };
        }
    });

    // match_arms_for_field_generator_func.extend(quote! {
    //     "quic.payload" => {
    //         let result = quic__payload(field.arg.unwrap() as usize / 8);
    //         let size_in_bits = result.len() * 8;
    //         return (result, size_in_bits);
    //     }
    // });

    funcs.push(quote! {
        // let mut custom_field_gen_funcs: HashMap<String, Box<dyn Fn(&FieldJson, &Vec<FieldJson>) -> Result<(Vec<u8>, usize), String>>> = HashMap::new();
        pub fn generate_field_func(field: &FieldJson, fields: &Vec<FieldJson>) -> Result<(Vec<u8>, usize), String> {
            match field.field.as_ref().unwrap_or_else(|| panic!("failed to get field.field.as_ref()")).as_str() {
                // "quic.payload" => { 
                //     let result = quic__payload(field.arg.unwrap() as usize / 8);
                //     let size_in_bits = result.len() * 8;
                //     return (result, size_in_bits);
                // }
                // "quic.remaining_payload" => { 
                //     let result = quic__remaining_payload(field.arg.unwrap() as usize  / 8);
                //     let size_in_bits = result.len() * 8;
                //     return (result, size_in_bits);
                // }
                #match_arms_for_field_generator_func
                field_name => {
                    return Err(format!("generate_field_func failed to match field type {} to generator function", field_name));
                    // panic!("generate_field_func failed to match field type to generator function");
                }
            }

            // (vec![], 0) 
        }
    });

    // funcs.push(quote! {
    //     macro_rules! generate_field {
    //         // ($func_name:ident, $( $args:expr ),* ) => {
    //         //     $func_name($( $args ),*)
    //         // };
    //         ($field_name:expr, $( $args:expr ),* ) => {
    //             match $field_name {
    //                 #match_arms
    //                 _ => println!("can't match in macro"),
    //             }
    //         };
    //     }
    // });

    let func_spec_file_hash = hasher.finish();
    let tokens = quote! {
        #(#funcs)*

        pub const FUNC_SPEC_FILE_HASH: u64 = #func_spec_file_hash;
    };

    // If running in debug mode then write the generated code to a file for easier debugging.
    #[cfg(debug_assertions)]
    {
        let token_string = tokens.to_string();
        let formatted = prettyplease::unparse(
            &syn::parse_file(&token_string).expect("Failed to parse token stream for debugging"),
        );
        // print!("{}", formatted);
        let path = std::path::Path::new("code_gen_output_debug.rs");
        let mut file =
            std::fs::File::create(&path).expect("failed to create output file for code gen debug");
        file.write_all(formatted.as_bytes())
            .expect("failed to write output file for code gen debug")
    }

    tokens
}

#[cfg(test)]
mod tests {
    use proc_macro::TokenTree;

    use super::*;

    #[test]
    #[should_panic]
    fn test_code_gen_impl_with_empty_string_should_panic() {
        code_gen_impl("");
    }

    #[test]
    #[should_panic]
    fn test_code_gen_impl_with_empty_array_should_panic() {
        code_gen_impl("[]");
    }

    #[test]
    #[should_panic]
    fn test_code_gen_impl_with_duplicate_field_names_should_panic() {
        code_gen_impl(
            r#"[
            {
                "output": "some.field.name"
            },
            {
                "output": "some.field.name"
            }
        ]"#,
        );
    }

    /// WARNING: this test is pretty brittle. It's just checking that the second Ident is the
    /// function name, i.e. the macro is producing something like
    ///     fn my__func__name ...
    /// If the implementation of the macro changes this test could easily break while not actually
    /// affecting the production code. Ideally find a better way to test this, but I don't know
    /// enough about all this proc macro craziness yet to figure it out right now.
    #[test]
    fn test_code_gen_impl_field_with_multiple_periods() {
        let result = code_gen_impl(
            r#"
            [{
                "output": "rtp.ext.rfc5285.data",
                "operation": "encrypted"
            }]"#,
        );

        let result_string = result.to_string();
        let mut token_parser = proc_macro_utils::TokenParser::new(result);
        let func = token_parser.peek_n(1).unwrap();
        match func {
            proc_macro2::TokenTree::Ident(func) => assert_eq!(
                func.to_string(),
                String::from("rtp__ext__rfc5285__data"),
                "{:?}",
                result_string
            ),
            _ => panic!("expected an Ident containing the function name"),
        }
    }

    #[test]
    fn test_code_gen_impl_length_field() {
        let result = code_gen_impl(
            r#"
            [{
                "output": "proto.length",
                "operation": "length",
                "args": [
                    "proto.target"
                ]
            }]"#,
        );

        let result_string = result.to_string();
        let mut token_parser = proc_macro_utils::TokenParser::new(result);
        let func = token_parser.peek_n(1).unwrap();
        match func {
            proc_macro2::TokenTree::Ident(func) => assert_eq!(
                func.to_string(),
                String::from("proto__length"),
                "{:?}",
                result_string
            ),
            _ => {
                println!("Token parser tokens:");
                for token in token_parser {
                    println!("{:?}", token);
                }
                panic!("expected an Ident containing the function name");
            }
        }
    }
}
