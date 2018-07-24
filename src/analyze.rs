use std::collections::HashSet;

#[allow(unused_imports)] // TODO
use syn::{
    FnArg, Ident, ItemTrait, Lifetime, MethodSig, Pat, PatIdent, TraitItem, TraitItemMethod,
    TraitItemType, TraitItemConst, TypeParamBound, Type, Generics, ArgSelfRef, ReturnType, Path,
    Member, PathArguments, GenericArgument, LifetimeDef, Block,
    visit::{Visit, visit_item_trait},
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
    // Define the visitor that just collects names
    struct IdentCollector<'ast> {
        ty_names: HashSet<&'ast Ident>,
        lt_names: HashSet<&'ast Ident>,
    }

    impl<'ast> Visit<'ast> for IdentCollector<'ast> {
        fn visit_ident(&mut self, i: &'ast Ident) {
            self.ty_names.insert(i);
        }

        fn visit_lifetime(&mut self, lt: &'ast Lifetime) {
            self.lt_names.insert(&lt.ident);
        }

        // Visiting a block just does nothing. It is the default body of a method
        // in the trait. But since that block won't be in the impl block, we can
        // just ignore it.
        fn visit_block(&mut self, _: &'ast Block) {}
    }

    // Create the visitor and visit the trait
    let mut visitor = IdentCollector {
        ty_names: HashSet::new(),
        lt_names: HashSet::new(),
    };
    visit_item_trait(&mut visitor, trait_def);

    println!("{:#?}", visitor.ty_names.into_iter().map(|i| i.to_string()).collect::<Vec<_>>());
    println!("{:#?}", visitor.lt_names.into_iter().map(|i| i.to_string()).collect::<Vec<_>>());

    Ok(())
}
