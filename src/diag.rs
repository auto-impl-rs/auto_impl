//! This module has two purposes:
//!
//! 1. Provide the convenienve method `emit_with_attr_note` and add it via
//!    extension trait to `Diagnostic`.
//!
//! 2. Make `Diagnostic` work on stable by providing an own `Diagnostic` type
//!    that prints the messages in a less-nice way. That way, other modules
//!    don't have to worry about the stable/nightly distinction. `SpanExt` is
//!    an extension trait that adds the `err()` method to the `Span` type. That
//!    method works exactly like `Span::error()` but returns "our"
//!    `Diagnostic`. Other modules can simply `use diag::SpanExt` and use
//!    `.err()` on spans.
//!

use proc_macro::{Span, TokenStream};


/// Extension trait that adds a convenience method to `Diagnostic`. This is
/// simply to reduce duplicate code in other modules.
pub trait DiagnosticExt {
    /// Helper function to add a note to the diagnostic (with a span pointing
    /// to the `auto_impl` attribute) and emit the error. An `Err(())` is
    /// always returned.
    fn emit_with_attr_note<T>(self) -> Result<T, ()>;
}

impl DiagnosticExt for Diagnostic {
    fn emit_with_attr_note<T>(self) -> Result<T, ()> {
        self.span_note(Span::call_site(), "auto-impl requested here")
            .emit();

        Err(())
    }
}


// ==============================================================
// Logic for stable/nightly mode starts here.
//
// First, we define a `Diagnostic` type. If we compile with the `nightly`
// feature, it's simple a typedef to `proc_macro::Diagnostic`. If we don't
// compile in nightly mode, we can't use that type, since it's still unstable.
// So in that case, we define our own type that tries to mimic the original
// `Diagnostic`.

#[cfg(feature = "nightly")]
crate type Diagnostic = ::proc_macro::Diagnostic;

#[cfg(not(feature = "nightly"))]
crate struct Diagnostic {
    span: Span,
    msg: String,
}

// We provide the methods that `proc_macro::Diagnostic` also has here. Or
// rather: we only implement the subset that this crate actually uses.
//
// When we're not on the nightly compiler, we can't show a nice error. So how
// do we show the error then? The idea is to generate a token stream that
// contains `compile_error!(msg)` macro invocations. This macro is part of the
// standard library and emits `msg` as error. This is fairly useful for our
// case. However, a big limitation is that we can only emit one message. So in
// order to also show notes later added to the `Diagnostic`, we simply add
// "note: {the_note}" to the error string. This is crude and ugly, but it
// works.
//
// What about spans? Good question! Spans are important, but without a proper
// `Diagnostic` API, we can't properly support spans on errors and notes. The
// compiler will point to the `compile_error!()` invocation we generate. But we
// can use another hack to improve the situation slightly! On the token stream
// (containing `compile_error!()`) we generate, we can modify the spans of the
// individual token trees. If we set all spans to the span the error originates
// from, the compiler thinks that the `compile_error!()` code snippet has the
// span from the actual error source. That means that the error message will
// point to the actual error source!
//
// There is only a small problem: this only works when we get a proper span.
// Sadly, on stable, we can only get correct spans for individual token trees,
// not even token streams. We can't combine spans. As a consequence, spans are
// only correct if they come directly from a `TokenTree`. In general, errors
// coming from the `proxy` module have proper spans while errors from other
// modules don't have proper spans (on stable!). "Not proper" means that the
// span is simply `call_site()` -- it points to the `#[auto_impl()]` attribute.
//
// It could be worse, but it's simply true: for good error messages, nightly is
// required.
#[cfg(not(feature = "nightly"))]
impl Diagnostic {
    crate fn note(mut self, msg: impl Into<String>) -> Diagnostic {
        self.msg += &format!("\n\nnote: {}", msg.into());
        self
    }

    crate fn span_note(mut self, _: Span, msg: impl Into<String>) -> Diagnostic {
        // With out span fake method, we can only handle one span. We take the
        // one of the original error and ignore additional ones.
        self.msg += &format!("\n\nnote: {}", msg.into());
        self
    }

    crate fn emit(self) {
        // Create the error token stream that contains the `compile_error!()`
        // invocation.
        let msg = &self.msg;
        let tokens = TokenStream::from(quote! {
            compile_error!(#msg);
        });

        // Set the span of each token tree to the span the error originates
        // from.
        let tokens = tokens.into_iter()
            .map(|mut tt| {
                tt.set_span(self.span);
                tt
            })
            .collect();

        // Push it to the global list of error streams
        ERROR_TOKENS.with(|toks| {
            toks.borrow_mut().push(tokens)
        });
    }
}

// Another problem with our `Diagnostic` hack on stable: the real
// `Diagnostic::emit()` doesn't return anything and modifies global state (it
// prints directly to stdout). We can't simply print! In our case it would be
// correct to pass a `TokenStream` ass the `Err()` variant of a result back up
// the stack and display it at the end. Two problems with that approach:
//
// - That's not how this application was build. Instead, it's build with the
//   future `proc_macro` API in mind. And we wouldn't want to change everything
//   back once it's stable.
// - On nightly, we don't want to pass TokenStreams up the stack. We can't have
//   a completely different structure on nightly vs. on stable.
//
// Thus, we just "simulate" the original `emit()` by also modifying global
// state. We simply have a list of error token streams. This list is added to
// the final token stream at the end (in case of an error). It's not a very
// nice solution, but it's only a hack while stable doesn't offer something
// proper.
#[cfg(not(feature = "nightly"))]
use std::cell::RefCell;

#[cfg(not(feature = "nightly"))]
thread_local! {
    static ERROR_TOKENS: RefCell<Vec<TokenStream>> = RefCell::new(vec![]);
}

/// On stable, we just copy the error token streams from the global variable.
#[cfg(not(feature = "nightly"))]
crate fn error_tokens() -> TokenStream {
    ERROR_TOKENS.with(|toks| toks.borrow().iter().cloned().collect())
}

/// On nightly, we don't use and don't have a strange global variable. Instead,
/// we just return an empty token stream. That's not a problem because all of
/// our errors were already printed.
#[cfg(feature = "nightly")]
crate fn error_tokens() -> TokenStream {
    TokenStream::new()
}

/// Extension trait to add the `err()` method to `Span`. This makes it easy to
/// start a `Diagnostic` from a span.
crate trait SpanExt {
    fn err(self, msg: impl Into<String>) -> Diagnostic;
}

impl SpanExt for Span {
    #[cfg(feature = "nightly")]
    fn err(self, msg: impl Into<String>) -> Diagnostic {
        Diagnostic::spanned(self, ::proc_macro::Level::Error, msg)
    }

    #[cfg(not(feature = "nightly"))]
    fn err(self, msg: impl Into<String>) -> Diagnostic {
        Diagnostic {
            span: self,
            msg: msg.into(),
        }
    }
}
