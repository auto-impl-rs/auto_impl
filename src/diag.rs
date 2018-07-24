use proc_macro::{Diagnostic, Span};


pub trait DiagnosticExt {
    /// Helper function to add a note to the diagnostic (with a span pointing
    /// to the `auto_impl` attribute) and emit the error. Additionally,
    /// `Err(())` is always returned.
    fn emit_with_attr_note<T>(self) -> Result<T, ()>;
}

impl DiagnosticExt for Diagnostic {
    fn emit_with_attr_note<T>(self) -> Result<T, ()> {
        self.span_note(Span::call_site(), "auto-impl requested here")
            .emit();

        Err(())
    }
}
