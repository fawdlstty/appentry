use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;

#[proc_macro_attribute]
pub fn appentry(args: TokenStream, input: TokenStream) -> TokenStream {
    let input_fn = syn::parse_macro_input!(input as syn::ItemFn);
    let fn_sig = &input_fn.sig;

    // Parse macro arguments
    let args_str = args.to_string();
    let is_default = args_str.contains("default");

    // Extract function name
    let fn_name = fn_sig.ident.to_string();
    let fn_ident = fn_sig.ident.clone();

    // Extract argument information
    let mut arg_names = Vec::new();
    let mut arg_types = Vec::new();
    let mut arg_descs = Vec::new();

    // Extract doc comments from the function to find parameter descriptions
    let doc_comments = extract_doc_comments(&input_fn.attrs);

    // Extract function description from doc comments (first non-whitespace line)
    let func_desc = extract_func_description(&doc_comments);

    for arg in &fn_sig.inputs {
        if let syn::FnArg::Typed(syn::PatType { pat, ty, .. }) = arg {
            let arg_name = match pat.as_ref() {
                syn::Pat::Ident(ident) => ident.ident.to_string(),
                _ => "_".to_string(),
            };
            let arg_type_str = quote! { #ty }.to_string();

            // Look for description of this parameter in doc comments
            let arg_desc = extract_param_description(&doc_comments, &arg_name);

            arg_names.push(arg_name);
            arg_types.push(arg_type_str);
            arg_descs.push(arg_desc);
        }
    }

    // Convert to static arrays
    let arg_count = arg_names.len();
    let arg_names_literals: Vec<syn::LitStr> = arg_names
        .iter()
        .map(|name| syn::LitStr::new(name, Span::call_site()))
        .collect();
    let arg_types_literals: Vec<syn::LitStr> = arg_types
        .iter()
        .map(|ty| syn::LitStr::new(ty, Span::call_site()))
        .collect();
    let arg_descs_literals: Vec<proc_macro2::TokenStream> = arg_descs
        .iter()
        .map(|desc_opt| match desc_opt {
            Some(desc) => {
                let lit_str = syn::LitStr::new(desc, Span::call_site());
                quote! { Some(#lit_str) }
            }
            None => quote! { None },
        })
        .collect();

    // Generate a wrapper function that handles arguments
    let wrapper_fn_name = syn::Ident::new(&format!("appentry_{}", fn_name), Span::call_site());

    // Process the original function's parameters to generate appropriate argument extraction
    let mut inputs_with_names = Vec::new();
    for (_i, input) in fn_sig.inputs.iter().enumerate() {
        if let syn::FnArg::Typed(syn::PatType { pat, ty, .. }) = input {
            if let syn::Pat::Ident(ident) = pat.as_ref() {
                let arg_name = &ident.ident;
                let arg_name_str = arg_name.to_string();
                let short_arg = format!("-{}", arg_name_str.chars().next().unwrap_or('_'));
                let long_arg = format!("--{}", arg_name_str);

                // Check if the type is bool to handle differently
                let is_bool = if let syn::Type::Path(type_path) = ty.as_ref() {
                    type_path
                        .path
                        .segments
                        .last()
                        .map_or(false, |seg| seg.ident == "bool")
                } else {
                    false
                };

                inputs_with_names.push((
                    arg_name.clone(),
                    ty.clone(),
                    short_arg,
                    long_arg,
                    is_bool,
                ));
            }
        }
    }

    let param_processing: Vec<proc_macro2::TokenStream> = inputs_with_names
        .iter()
        .map(|(arg_ident, _, short_arg, long_arg, is_bool)| {
            let short_arg_lit = syn::LitStr::new(short_arg, Span::call_site());
            let long_arg_lit = syn::LitStr::new(long_arg, Span::call_site());
            if *is_bool {
                // For boolean arguments, we can just check if the flag exists
                quote! {
                    let #arg_ident = ::appentry::get_arg_from_name(args, &[#short_arg_lit, #long_arg_lit]);
                }
            } else {
                quote! {
                    let #arg_ident = ::appentry::get_arg_from_name(args, &[#short_arg_lit, #long_arg_lit]);
                }
            }
        })
        .collect();

    // Generate parameter extraction for async context (to avoid lifetime issues)
    let async_param_processing: Vec<proc_macro2::TokenStream> = inputs_with_names
        .iter()
        .map(|(arg_ident, _ty, short_arg, long_arg, is_bool)| {
            let short_arg_lit = syn::LitStr::new(short_arg, Span::call_site());
            let long_arg_lit = syn::LitStr::new(long_arg, Span::call_site());
            // For async context, we'll call the same function but in async context
            if *is_bool {
                quote! {
                    let #arg_ident = ::appentry::get_arg_from_name(args, &[#short_arg_lit, #long_arg_lit]);
                }
            } else {
                quote! {
                    let #arg_ident = ::appentry::get_arg_from_name(args, &[#short_arg_lit, #long_arg_lit]);
                }
            }
        })
        .collect();

    let arg_refs: Vec<syn::Ident> = inputs_with_names
        .iter()
        .map(|(name, _, _, _, _)| name.clone())
        .collect();

    // Check if the return type is Result<_, _>
    let has_result_return = if let syn::ReturnType::Type(_, ty) = &fn_sig.output {
        if let syn::Type::Path(type_path) = ty.as_ref() {
            type_path
                .path
                .segments
                .last()
                .map_or(false, |segment| segment.ident == "Result")
        } else {
            false
        }
    } else {
        false
    };

    // Check if the original function is async
    let is_async = fn_sig.asyncness.is_some();

    // Generate the inventory submission code
    let fn_name_literal = syn::LitStr::new(&fn_name, Span::call_site());
    let original_function = &input_fn;

    // Define the wrapper function based on whether the original function is async
    let wrapper_function_definition = if is_async {
        let call_with_result_handling = match has_result_return {
            true => quote! { #fn_ident(#(#arg_refs),*).await?; },
            false => quote! { #fn_ident(#(#arg_refs),*).await; },
        };
        let async_wrapper = quote! {
            fn #wrapper_fn_name(args: &mut std::collections::HashMap<String, Option<String>>) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>>>> {
                #(#async_param_processing)* // Process parameters synchronously
                Box::pin(async move {
                    #call_with_result_handling
                    Ok(())
                })
            }
        };
        async_wrapper
    } else {
        let call_with_result_handling = match has_result_return {
            true => quote! { #fn_ident(#(#arg_refs),*)?; },
            false => quote! { #fn_ident(#(#arg_refs),*); },
        };
        quote! {
            fn #wrapper_fn_name(args: &mut std::collections::HashMap<String, Option<String>>) -> anyhow::Result<()> {
                #(#param_processing)*
                #call_with_result_handling
                Ok(())
            }
        }
    };

    // Define the method type based on whether the original function is async
    let method_type = if is_async {
        quote! {
            ::appentry::AppEntryMethod::Async(#wrapper_fn_name)
        }
    } else {
        quote! {
            ::appentry::AppEntryMethod::Sync(#wrapper_fn_name)
        }
    };

    let expanded = quote! {
        // The original function
        #original_function

        // Create a wrapper function to handle arguments
        #wrapper_function_definition

        // Submit the function info to inventory directly
        ::appentry::inventory::submit! {
            {
                const ARGS: [::appentry::ArgInfo; #arg_count] = [
                    #(
                        ::appentry::ArgInfo::new_with_desc(
                            #arg_names_literals,
                            #arg_types_literals,
                            #arg_descs_literals
                        ),
                    )*
                ];
                ::appentry::FunctionInfo::new(
                    #fn_name_literal,
                    #is_default,
                    #func_desc,
                    &ARGS,
                    #method_type
                )
            }
        }
    };

    //panic!("{}", expanded.to_string());
    expanded.into()
}

fn extract_doc_comments(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| {
            if let syn::Meta::NameValue(syn::MetaNameValue {
                value:
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit_str),
                        ..
                    }),
                ..
            }) = attr.meta.clone()
            {
                Some(lit_str.value())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_param_description(doc_comments: &str, param_name: &str) -> Option<String> {
    // Look for parameter descriptions in common Rust doc comment formats
    let lines: Vec<&str> = doc_comments.lines().collect();

    // First, look for patterns like "* `param_name` - description" (Rust standard format)
    for line in &lines {
        let trimmed = line.trim();
        if trimmed.contains(param_name) && (trimmed.contains('`') && trimmed.contains('-')) {
            // Look for format like "* `name` - The name of the person to greet"
            if let Some(start_pos) = trimmed.find(&format!("`{}`", param_name)) {
                // Find the dash after the parameter name
                if let Some(dash_pos) = trimmed[start_pos..].find(" - ") {
                    let full_dash_pos = start_pos + dash_pos + 3; // +3 for " - "
                    let desc = trimmed[full_dash_pos..].trim();
                    if !desc.is_empty() {
                        return Some(desc.to_string());
                    }
                }
            }
        }
    }

    // Look for patterns like "param_name: description"
    for line in &lines {
        let trimmed = line.trim();
        if trimmed.starts_with(param_name) && trimmed.contains(':') {
            let colon_pos = trimmed.find(':').unwrap();
            let desc = trimmed[colon_pos + 1..].trim();
            if !desc.is_empty() {
                return Some(desc.to_string());
            }
        }
    }

    // Look for patterns like "# Arguments" section
    let mut in_arguments_section = false;
    for line in &lines {
        let trimmed = line.trim();

        if trimmed.to_lowercase().contains("arguments") && trimmed.starts_with('#') {
            in_arguments_section = true;
            continue;
        }

        if trimmed.starts_with('#') && !trimmed.to_lowercase().contains("arguments") {
            // We've moved to a different section
            in_arguments_section = false;
        }

        if in_arguments_section {
            if trimmed.contains(param_name) && (trimmed.contains('`') && trimmed.contains('-')) {
                if let Some(start_pos) = trimmed.find(&format!("`{}`", param_name)) {
                    if let Some(dash_pos) = trimmed[start_pos..].find(" - ") {
                        let full_dash_pos = start_pos + dash_pos + 3;
                        let desc = trimmed[full_dash_pos..].trim();
                        if !desc.is_empty() {
                            return Some(desc.to_string());
                        }
                    }
                }
            }
        }
    }

    None
}

fn extract_func_description(doc_comments: &str) -> Option<proc_macro2::TokenStream> {
    // Split the doc comments into lines and find the first meaningful description
    // Skip any empty lines or lines that are just whitespace
    let lines: Vec<&str> = doc_comments.lines().collect();

    for line in lines {
        let trimmed = line.trim();
        // Skip empty lines or lines that start with '#' (headers) or '*' (list items)
        if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with('*') {
            // This is likely the function description
            // Avoid returning argument descriptions
            if !trimmed.to_lowercase().contains("arguments")
                && !trimmed.to_lowercase().contains("params")
                && !trimmed.to_lowercase().contains(":")
            {
                let lit_str = syn::LitStr::new(trimmed, Span::call_site());
                return Some(quote! { Some(#lit_str) });
            }
        }
    }
    Some(quote! { None })
}
