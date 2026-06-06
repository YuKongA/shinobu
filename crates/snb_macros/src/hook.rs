use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::Ident;

use crate::common::{MetaArgs, name_method, parse_fn, priority_method};

/// `#[hook(name = "log_hook", kind = HookType::All, priority = 0)]` on a free fn:
///
/// ```ignore
/// #[hook(name = "log_hook", kind = HookType::All)]
/// fn log_hook(event: &mut Event) -> anyhow::Result<()> { ... }
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
    let priority_fn = priority_method(&args);
    let kind = args.require("kind", &func.sig)?;

    let ty = Ident::new(&format!("__SnbHook_{fn_name}"), Span::call_site());

    Ok(quote! {
        #func

        #[doc(hidden)]
        #[derive(Clone, Copy)]
        struct #ty;

        impl ::snb_core::hook::Hook for #ty {
            #name_fn
            fn hook_type(&self) -> ::snb_core::hook::HookType { #kind }
            #priority_fn
            fn execute(&self, event: &mut ::snb_core::event::Event) -> ::anyhow::Result<()> {
                #fn_name(event)
            }
        }

        ::snb_core::registry::submit! {
            ::snb_core::registry::HookRegistration {
                factory: || ::std::sync::Arc::new(#ty),
            }
        }
    })
}
