use std::collections::HashSet;

#[allow(unused_imports)] // TODO
use syn::{
    FnArg, Ident, ItemTrait, Lifetime, MethodSig, Pat, PatIdent, TraitItem, TraitItemMethod,
    TraitItemType, TraitItemConst, TypeParamBound, Type, Generics, ArgSelfRef, ReturnType, Path,
    Member, PathArguments, GenericArgument,
};

use crate::{
    diag::DiagnosticExt,
    spanned::Spanned,
};


/// We need to introduce our own type and lifetime parameter. Regardless of
/// what kind of hygiene we use for the parameter, it would be nice (for docs
/// and compiler errors) if the names are as simple as possible ('a and T, for
/// example).
///
/// This function searches for names that we can use. Such a name must not
/// conflict with any other name we'll use in the `impl` block. Luckily, we
/// know all those names in advance. Names we'll (possibly) use:
/// - the trait name
/// - any type/lifetime parameters of the trait
/// - super-trait names
/// - any names used in any of:
///   - trait method signatures (excluding body of default methods!)
///   - associated types (name and parameters, in the case of GATs)
///   - associated consts (name and type)
///
/// The idea is to collect all names used in any of the things listed above,
/// store them in a set and later check which name we can use. We don't collect
/// any "special" names like `'static` or `?Sized`, as we are not trying to
/// use those for our parameters anyway.
pub(crate) fn find_suitable_param_names(trait_def: &ItemTrait) -> Result<(), ()> {
    let mut used_ty_names = HashSet::new();
    let mut used_lt_names = HashSet::new();

    collect_names_in_trait(trait_def, &mut used_ty_names, &mut used_lt_names)?;


    println!("{:#?}", used_ty_names.into_iter().map(|i| i.to_string()).collect::<Vec<_>>());
    println!("{:#?}", used_lt_names.into_iter().map(|i| i.to_string()).collect::<Vec<_>>());

    Ok(())
}

/// Recursively traverses all parts of the given trait and searches for idents
/// that will also be used in the `impl` block. All idents used as a lifetime
/// are stored in `lt_names`, all other idents in `ty_names`.
fn collect_names_in_trait<'a>(
    trait_def: &'a ItemTrait,
    ty_names: &mut HashSet<&'a Ident>,
    lt_names: &mut HashSet<&'a Ident>,
) -> Result<(), ()> {
    // Name of the trait
    ty_names.insert(&trait_def.ident);

    // Type and lifetime parameters of the trait
    collect_names_in_generics(&trait_def.generics, ty_names, lt_names)?;

    // Super traits
    for bound in &trait_def.supertraits {
        collect_names_in_ty_param_bound(bound, ty_names, lt_names);
    }

    // Trait items
    for item in &trait_def.items {
        match item {
            TraitItem::Const(c) => {
                ty_names.insert(&c.ident);
                collect_names_in_type(&c.ty, ty_names, lt_names)?;

                // We don't care about the default value, as it won't be in our
                // impl block.
            }

            TraitItem::Method(method) => {
                // Function name
                ty_names.insert(&method.sig.ident);

                let decl = &method.sig.decl;

                // Generic parameters
                collect_names_in_generics(&decl.generics, ty_names, lt_names)?;

                // Function arguments
                for arg in &decl.inputs {
                    collect_names_in_fn_arg(arg, ty_names, lt_names)?;
                }

                // Return type
                if let ReturnType::Type(_, ty) = &decl.output {
                    collect_names_in_type(ty, ty_names, lt_names)?;
                }
            }

            TraitItem::Type(ty) => {
                ty_names.insert(&ty.ident);
                collect_names_in_generics(&ty.generics, ty_names, lt_names)?;

                // We don't care about the bounds as we won't have them in our
                // impl block.
            }

            TraitItem::Macro(mac) => {
                // We cannot resolve the macro invocation and thus cannot know
                // if it adds additional items to the trait. Thus, we have to
                // give up.
                return mac.span()
                    .error(
                        "traits with macro invocations in their bodies are not \
                         supported by auto_impl"
                    )
                    .emit_with_attr_note();
            }

            TraitItem::Verbatim(v) => {
                // I don't quite know when this happens, but it's better to
                // notify the user with a nice error instead of panicking.
                return v.span()
                    .error("unexpected 'verbatim'-item (auto-impl doesn't know how to handle it)")
                    .emit_with_attr_note();
            }
        }
    }

    Ok(())
}

fn collect_names_in_fn_arg<'a>(
    arg: &'a FnArg,
    ty_names: &mut HashSet<&'a Ident>,
    lt_names: &mut HashSet<&'a Ident>,
) -> Result<(), ()> {
    match arg {
        // Only the lifetime might be interesting
        FnArg::SelfRef(ArgSelfRef { lifetime: Some(lt), .. }) => {
            lt_names.insert(&lt.ident);
        }

        FnArg::Captured(arg) => {
            collect_names_in_pattern(&arg.pat, ty_names, lt_names)?;
            collect_names_in_type(&arg.ty, ty_names, lt_names)?;
        }

        FnArg::Inferred(pat) => {
            collect_names_in_pattern(pat, ty_names, lt_names)?;
        }

        FnArg::Ignored(ty) => {
            collect_names_in_type(ty, ty_names, lt_names)?;
        }

        // `SelfValue` and `SelfRef` without lifetime don't have names of
        // interest.
        _ => {}
    }

    Ok(())
}

fn collect_names_in_path<'a>(
    path: &'a Path,
    ty_names: &mut HashSet<&'a Ident>,
    lt_names: &mut HashSet<&'a Ident>,
) -> Result<(), ()> {
    // Of the path segments' names, only the first name could potentiall lead
    // to name collisions, but only if the path doesn't start with a leading
    // double-colon.
    if path.leading_colon.is_none() {
        if let Some(segment) = path.segments.first() {
            ty_names.insert(&segment.value().ident);
        }
    }

    // Apart from the names, we do care about the arguments of all segments
    for seg in &path.segments {
        match &seg.arguments {
            PathArguments:: None => {}
            PathArguments::AngleBracketed(args) => {
                for arg in &args.args {
                    match arg {
                        // These lifetimes have to be declared somewhere else
                        // (or be 'static), so we already found those names.
                        GenericArgument::Lifetime(_) => {}

                        GenericArgument::Type(ty) => {
                            collect_names_in_type(ty, ty_names, lt_names)?;
                        }

                        GenericArgument::Binding(binding) => {
                            // The ident of the binding can't lead to naming collisions
                            collect_names_in_type(&binding.ty, ty_names, lt_names)?;
                        }

                        GenericArgument::Const(c) => {
                            return c.span()
                                .error("this pattern is not supported by #[auto-impl]")
                                .emit_with_attr_note();
                        }
                    }
                }
            }
            PathArguments::Parenthesized(args) => {

            }
        }
    }

    Ok(())
}

fn collect_names_in_pattern<'a>(
    pat: &'a Pat,
    ty_names: &mut HashSet<&'a Ident>,
    lt_names: &mut HashSet<&'a Ident>,
) -> Result<(), ()> {
    match pat {
        // foo @ <sub-pattern>
        Pat::Ident(pat) => {
            ty_names.insert(&pat.ident);

            if let Some((_, pat)) = &pat.subpat {
                collect_names_in_pattern(pat, ty_names, lt_names)?;
            }
        }

        // (a, b, .., c, _)
        Pat::Tuple(pat) => {
            for pat in pat.front.iter().chain(&pat.back) {
                collect_names_in_pattern(pat, ty_names, lt_names)?;
            }
        }

        // Foo { a, b: <sub-pattern>, .. }
        Pat::Struct(pat) => {
            collect_names_in_path(&pat.path, ty_names, lt_names)?;
            for pat in &pat.fields {
                if let Member::Named(name) = &pat.member {
                    ty_names.insert(name);
                }

                collect_names_in_pattern(&pat.pat, ty_names, lt_names)?;
            }
        }

        // Foo(a, b, _)
        Pat::TupleStruct(pat) => {
            collect_names_in_path(&pat.path, ty_names, lt_names)?;
            for pat in pat.pat.front.iter().chain(&pat.pat.back) {
                collect_names_in_pattern(pat, ty_names, lt_names)?;
            }
        }

        // box <sub-pattern>
        Pat::Box(pat) => {
            collect_names_in_pattern(&pat.pat, ty_names, lt_names)?;
        }

        // &mut <sub-pattern>
        Pat::Ref(pat) => {
            collect_names_in_pattern(&pat.pat, ty_names, lt_names)?;

        }

        // [a, b.., <sub-pattern>]
        Pat::Slice(pat) => {
            for pat in pat.front.iter().chain(&pat.back) {
                collect_names_in_pattern(pat, ty_names, lt_names)?;
            }
            if let Some(pat) = &pat.middle {
                collect_names_in_pattern(pat, ty_names, lt_names)?;
            }
        }

        // These patterns do not contain any names of interest
        Pat::Wild(_) | Pat::Lit(_)  => {}

        // These pattern are not allowed in trait method arguments or we simply
        // can't handle them.
        Pat::Macro(_) | Pat::Path(_) | Pat::Verbatim(_) | Pat::Range(_) => {
            return pat.span()
                .error("this pattern is not supported by #[auto-impl]")
                .emit_with_attr_note();
        },
    }

    Ok(())
}

fn collect_names_in_ty_param_bound<'a>(
    bound: &'a TypeParamBound,
    ty_names: &mut HashSet<&'a Ident>,
    lt_names: &mut HashSet<&'a Ident>,
) -> Result<(), ()> {
    // Lifetime bounds don't introduce new names (except maybe 'static, but we
    // don't care about that).
    if let TypeParamBound::Trait(t) = bound {
        // HRTBs (`for<'a, 'b>`) introduces new names
        if let Some(lifetimes) = &t.lifetimes {
            let idents = lifetimes.lifetimes
                .iter()
                .map(|lt_def| &lt_def.lifetime.ident);

            lt_names.extend(idents);
        }

        // Path ("name") of super trait
        collect_names_in_path(&t.path, ty_names, lt_names)?;
    }

    Ok(())
}

fn collect_names_in_generics<'a>(
    generics: &'a Generics,
    ty_names: &mut HashSet<&'a Ident>,
    lt_names: &mut HashSet<&'a Ident>,
) -> Result<(), ()> {
    // For type parameters, we only have to consider the names of the
    // parameters. Bounds are not important, because there are no "global"
    // lifetimes except for 'static. So all other lifetimes need to be declared
    // somewhere else (where we already spot them).
    lt_names.extend(generics.lifetimes().map(|l| &l.lifetime.ident));

    // Type bounds on the other hand need to be considered, because arbitrary
    // trait names can be used from the outside. The default value can also
    // be part of the `impl` block, if these are generics of a trait item.
    for param in generics.type_params() {
        ty_names.insert(&param.ident);

        for bound in &param.bounds {
            collect_names_in_ty_param_bound(bound, ty_names, lt_names);
        }

        if let Some(ty) = &param.default {
            collect_names_in_type(ty, ty_names, lt_names)?;
        }
    }

    Ok(())
}

fn collect_names_in_type<'a>(
    _ty: &'a Type,
    _ty_names: &mut HashSet<&'a Ident>,
    _lt_names: &mut HashSet<&'a Ident>,
) -> Result<(), ()> {
    // TODO

    Ok(())
}

#[cfg(test)]
mod tests {
    // use std::collections::HashSet;
    use proc_macro2::Span as Span2;

    use syn;

    use super::*;

    macro_rules! assert_set_eq {
        ($left:ident == [$($s:expr),*]) => {{
            #[allow(unused_mut)]
            let mut right = HashSet::new();
            $(
                right.insert(Ident::new($s, Span2::call_site()));
            )*

            assert_eq!($left.into_iter().cloned().collect::<HashSet<_>>(), right);
        }}
    }

    macro_rules! run {
        ($func:ident ($arg:expr , .., ..) ) => {{
            let mut ty_names = HashSet::new();
            let mut lt_names = HashSet::new();
            let res = $func($arg, &mut ty_names, &mut lt_names);
            (res, ty_names, lt_names)
        }};
        ($func:ident ($arg:expr , ..) ) => {{
            let mut ty_names = HashSet::new();
            let res = $func($arg, &mut ty_names);
            (res, ty_names)
        }};
    }

    // ==========================================================================================
    // ===== PATTERN
    // ==========================================================================================
    macro_rules! test_pattern {
        ($pat:expr, $names:tt) => {{
            let pat = syn::parse_str($pat).unwrap();
            let (res, ty_names, _) = run!(collect_names_in_pattern(&pat, .., ..));
            res.unwrap();

            assert_set_eq!(ty_names == $names);
        }}
    }

    #[test]
    fn pattern_simple() {
        test_pattern!("foo", ["foo"]);
        test_pattern!("foo @ bar", ["foo", "bar"]);
    }

    #[test]
    fn pattern_ignore() {
        test_pattern!("_", []);
    }

    #[test]
    fn pattern_literal() {
        test_pattern!("3.14", []);
        test_pattern!("false", []);
    }

    #[test]
    fn pattern_tuple() {
        test_pattern!("(a,)", ["a"]);
        test_pattern!("(a, b)", ["a", "b"]);
        test_pattern!("(a, b, _)", ["a", "b"]);
        test_pattern!("(a, _, c)", ["a", "c"]);
        test_pattern!("(a, ..)", ["a"]);
        test_pattern!("(a, .., x)", ["a", "x"]);
        test_pattern!("(_, b, .., x, _, z)", ["b", "x", "z"]);
    }

    #[test]
    fn pattern_struct() {
        test_pattern!("Foo {}", ["Foo"]);
        test_pattern!("Foo { .. }", ["Foo"]);
        test_pattern!("::Foo { .. }", []);
        test_pattern!("std::foo::Foo { .. }", ["std"]);
        test_pattern!("Foo { a, b }", ["Foo", "a", "b"]);
        test_pattern!("Foo { a, .. }", ["Foo", "a"]);
        test_pattern!("Foo { a: (x, y), b: _, .. }", ["Foo", "a", "x", "y", "b"]);
    }

    #[test]
    fn pattern_tuple_struct() {
        test_pattern!("Foo()", ["Foo"]);
        test_pattern!("Foo( .. )", ["Foo"]);
        test_pattern!("::Foo( .. )", []);
        test_pattern!("std::foo::Foo()", ["std"]);
        test_pattern!("Foo(a, b)", ["Foo", "a", "b"]);
        test_pattern!("Foo(a, ..)", ["Foo", "a"]);
        test_pattern!("Foo((x, y), _, ..)", ["Foo", "x", "y"]);
    }

    #[test]
    fn pattern_box() {
        test_pattern!("box _", []);
        test_pattern!("box a", ["a"]);
        test_pattern!("box (a, b)", ["a", "b"]);
    }

    #[test]
    fn pattern_ref() {
        test_pattern!("&_", []);
        test_pattern!("&a", ["a"]);
        test_pattern!("&(a, b)", ["a", "b"]);
        test_pattern!("&mut _", []);
        test_pattern!("&mut a", ["a"]);
        test_pattern!("&mut (a, b)", ["a", "b"]);
    }

    #[test]
    fn pattern_slice() {
        test_pattern!("[a]", ["a"]);
        test_pattern!("[a, b]", ["a", "b"]);
        test_pattern!("[a, b, _]", ["a", "b"]);
        test_pattern!("[a, _, c]", ["a", "c"]);
        test_pattern!("[a, ..]", ["a"]);
        test_pattern!("[a, .., x]", ["a", "x"]);
        test_pattern!("[_, b, .., x, _, z]", ["b", "x", "z"]);
        test_pattern!("[a, b.., c]", ["a", "b", "c"]);
        test_pattern!("[a, [x, y].., c]", ["a", "x", "y", "c"]);
    }


    // ==========================================================================================
    // ===== PATH
    // ==========================================================================================
    macro_rules! test_path {
        ($path:expr, $ty_names:tt, $lt_names:tt) => {{
            let path = syn::parse_str($path).unwrap();
            let (res, ty_names, lt_names) = run!(collect_names_in_path(&path, .., ..));
            res.unwrap();

            assert_set_eq!(ty_names == $ty_names);
            assert_set_eq!(lt_names == $lt_names);
        }}
    }

    #[test]
    fn path_simple() {
        test_path!("foo", ["foo"], []);
        test_path!("::foo", [], []);
        test_path!("foo::bar", ["foo"], []);
        test_path!("::foo::bar", [], []);
    }

    #[test]
    fn path_with_args() {
        test_path!("foo<bar>", ["foo"], []);
        test_path!("::foo<bar>", ["bar"], []);
        test_path!("foo::Bar<X>::Baz<Y>", ["foo", "X", "Y"], []);
    }


    // ==========================================================================================
    // ===== TY PARAM BOUND
    // ==========================================================================================
    #[test]
    fn ty_param_bound() {
        macro_rules! test {
            ($path:expr, $ty_names:tt, $lt_names:tt) => {{
                let path = syn::parse_str($path).unwrap();
                let (_, ty_names, lt_names) = run!(collect_names_in_ty_param_bound(&path, .., ..));

                assert_set_eq!(ty_names == $ty_names);
                assert_set_eq!(lt_names == $lt_names);
            }}
        }

        test!("Clone", ["Clone"], []);
        test!("Into<String>", ["Into", "String"], []);
        test!("foo::bar::Baz<X>::Zap<Y>", ["foo", "bar", "Baz", "X", "Zap", "Y"], []);
        test!("for<'a> Foo<'a>", ["Foo"], ["a"]);
        test!("for<'a, 'b> Foo<'a>", ["Foo"], ["a", "b"]);
        test!("'a", [], []);
        test!("'static", [], []);
    }

    // ==========================================================================================
    // ===== GENERICS
    // ==========================================================================================
    macro_rules! test_generics {
        ($path:expr, $ty_names:tt, $lt_names:tt) => {{
            let path = syn::parse_str($path).unwrap();
            let (res, ty_names, lt_names) = run!(collect_names_in_generics(&path, .., ..));
            res.unwrap();

            assert_set_eq!(ty_names == $ty_names);
            assert_set_eq!(lt_names == $lt_names);
        }}
    }

    #[test]
    fn generics_simple() {
        test_generics!("<T>", ["T"], []);
        test_generics!("<T, U>", ["T", "U"], []);
        test_generics!("<'a, T>", ["T"], ["a"]);
    }

    #[test]
    fn generics_bounds() {
        test_generics!("<T: Foo>", ["T", "Foo"], []);
        test_generics!("<T: Into<String>>", ["T", "Into", "String"], []);
        test_generics!("<'a: 'b, T: Foo>", ["T", "Foo"], ["a"]);
    }
}
