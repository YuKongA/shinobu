use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Expr, Ident};

use crate::common::{MetaArgs, name_method, parse_fn};

/// `#[command(name = "echo", aliases = ["say"])]` on a free function:
///
/// ```ignore
/// #[command(name = "echo", aliases = ["say"])]
/// fn echo(ctx: &CommandContext) -> anyhow::Result<()> { ... }
/// ```
pub fn expand(attr: TokenStream, item: TokenStream) -> TokenStream {
    match try_expand(attr, item) {
        Ok(ts) => ts,
        Err(e) => e.to_compile_error(),
    }
}

fn try_expand(attr: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    let args = MetaArgs::parse(attr)?;
    let func = parse_fn(item)?;
    let fn_name = &func.sig.ident;

    let name_fn = name_method(&args, &func.sig)?;
    let aliases_fn = match args.get("aliases") {
        Some(Expr::Array(arr)) => {
            let elems = arr.elems.iter();
            quote! { fn aliases(&self) -> ::std::vec::Vec<&str> { ::std::vec![ #( #elems ),* ] } }
        }
        Some(other) => {
            return Err(syn::Error::new_spanned(
                other,
                "`aliases` must be an array literal, e.g. aliases = [\"say\"]",
            ));
        }
        None => quote! {},
    };

    let ty = Ident::new(&format!("__SnbCommand_{fn_name}"), Span::call_site());

    Ok(quote! {
        #func

        #[doc(hidden)]
        #[derive(Clone, Copy)]
        struct #ty;

        impl ::snb_core::command::CommandHandler for #ty {
            #name_fn
            #aliases_fn
            fn execute(&self, ctx: &::snb_core::command::CommandContext) -> ::anyhow::Result<()> {
                #fn_name(ctx)
            }
        }

        ::snb_core::registry::submit! {
            ::snb_core::registry::CommandRegistration {
                factory: || ::std::sync::Arc::new(#ty),
            }
        }
    })
}
