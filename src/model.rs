use std::fmt;
use syn;
use quote::{Tokens, ToTokens};

pub struct Trait {
    ident: &'static str,
    tokens: Tokens,
}

impl Trait {
    pub fn new(ident: &'static str, tokens: Tokens) -> Self {
        Trait {
            ident: ident,
            tokens: tokens
        }
    }
}

impl fmt::Display for Trait {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.ident)
    }
}

impl ToTokens for Trait {
    fn to_tokens(&self, tokens: &mut Tokens) {
        tokens.append(&self.tokens);
    }
}

#[derive(Clone)]
struct Item {
    ident: syn::Ident, 
    attrs: Vec<syn::Attribute>,
}

pub struct AutoImpl {
    pub ident: syn::Ident,
    generics: syn::Generics,
    pub associated_types: Vec<AutoImplAssociatedType>,
    pub methods: Vec<AutoImplMethod>,
}

pub enum SelfArg {
    Ref(Option<syn::Lifetime>, syn::Mutability),
    Value(syn::Mutability),
}

pub struct AutoImplMethod {
    pub ident: syn::Ident,
    pub arg_self: Option<SelfArg>,
    pub arg_pats: Vec<syn::Pat>,
    pub arg_tys: Vec<syn::Ty>,
    pub output: Option<syn::Ty>,
    item_impl_factory: Box<Fn(syn::Block) -> Result<syn::TraitItem, String>>,
}

pub struct AutoImplAssociatedType {
    pub ident: syn::Ident,
    item_impl_factory: Box<Fn(syn::Ty) -> Result<syn::TraitItem, String>>,
}

impl AutoImpl {
    pub fn try_parse(item: syn::Item) -> Result<Self, String> {
        match item.node {
            syn::ItemKind::Trait(_, generics, _, items) => {
                let mut methods = Vec::new();
                let mut associated_types = Vec::new();

                for item in items {
                    let node = item.node;

                    let item = Item {
                        ident: item.ident,
                        attrs: item.attrs
                    };

                    match node {
                        syn::TraitItemKind::Method(sig, _) => {
                            let method = AutoImplMethod::try_parse(item, sig)?;
                            methods.push(method);
                        },
                        syn::TraitItemKind::Type(_, _) => {
                            let ty = AutoImplAssociatedType::try_parse(item)?;
                            associated_types.push(ty);
                        },
                        _ => Err("only methods and associated types are supported")?
                    }
                }

                Ok(AutoImpl {
                    ident: item.ident,
                    generics: generics,
                    associated_types: associated_types,
                    methods: methods
                })
            },
            _ => Err("expected a `trait`")?
        }
    }

    pub fn split_generics<'a>(&'a self) -> (syn::TyGenerics<'a>, &[syn::LifetimeDef], &[syn::TyParam], &[syn::WherePredicate]) {
        let (_, trait_tys, _) = self.generics.split_for_impl();
        let impl_lifetimes = &self.generics.lifetimes;
        let impl_tys = &self.generics.ty_params;
        let where_clauses = &self.generics.where_clause.predicates;

        (trait_tys, impl_lifetimes, impl_tys, where_clauses)
    }
}

impl AutoImplMethod {
    fn try_parse(item: Item, sig: syn::MethodSig) -> Result<Self, String> {
        let ident = item.ident.clone();
        let mut arg_self = None;
        let mut arg_pats = Vec::new();
        let mut arg_tys = Vec::new();

        for arg in &sig.decl.inputs {
            match *arg {
                syn::FnArg::SelfRef(ref lifetimes, ref mutability) => {
                    arg_self = Some(SelfArg::Ref(lifetimes.clone(), mutability.clone()));   
                },
                syn::FnArg::SelfValue(ref mutability) => {
                    arg_self = Some(SelfArg::Value(mutability.clone()));
                }
                syn::FnArg::Captured(ref pat, ref ty) => {
                    arg_pats.push(pat.clone());
                    arg_tys.push(ty.clone());
                },
                _ => Err("expected self or ident arg")?
            }
        }

        let output = match sig.decl.output {
            syn::FunctionRetTy::Ty(ref ty) => Some(ty.clone()),
            _ => None
        };

        let item_impl_factory = move |block| {
            let sig = sig.clone();
            let item = item.clone();

            let node = syn::TraitItemKind::Method(sig, Some(block));

            Ok(syn::TraitItem {
                ident: item.ident,
                attrs: vec![],
                node: node,
            })
        };

        Ok(AutoImplMethod {
            ident: ident,
            item_impl_factory: Box::new(item_impl_factory),
            arg_pats: arg_pats, 
            arg_tys: arg_tys,
            arg_self: arg_self,
            output: output
        })
    }

    pub fn build_impl_item<F>(&self, block_factory: F) -> Result<syn::TraitItem, String>
        where F: Fn(&Self) -> Tokens
    {
        let block_tokens = block_factory(&self);

        let block_expr = syn::parse_expr(&block_tokens.to_string())?;

        let block = match block_expr.node {
            syn::ExprKind::Block(_, block) => block,
            _ => Err("expected a block")?
        };

        (self.item_impl_factory)(block)
    }

    pub fn anonymous_arg_lifetimes(&self) -> Vec<syn::Ty> {
        self.arg_tys.iter()
            .map(|ty| {
                match *ty {
                    syn::Ty::Rptr(ref lifetime, ref ty) => {
                        let ty = ty.clone();

                        let lifetime = match lifetime {
                            &Some(ref l) if l.ident.as_ref() == "'static" => lifetime.to_owned(),
                            _ => None
                        };
                        
                        syn::Ty::Rptr(lifetime, ty)
                    },
                    ref ty @ _ => ty.clone()
                }
            })
            .collect()
    }
}

impl AutoImplAssociatedType {
    fn try_parse(item: Item) -> Result<Self, String> {
        let ident = item.ident.clone();
        let item_impl_factory = move |ty| {
            let item = item.clone();

            let node = syn::TraitItemKind::Type(vec![], Some(ty));

            Ok(syn::TraitItem {
                ident: item.ident,
                attrs: vec![],
                node: node,
            })
        };

        Ok(AutoImplAssociatedType {
            ident: ident,
            item_impl_factory: Box::new(item_impl_factory),
        })
    }

    pub fn build_impl_item<F>(&self, ty_factory: F) -> Result<syn::TraitItem, String>
        where F: Fn(&Self) -> Tokens
    {
        let ty_tokens = ty_factory(&self);

        let ty = syn::parse_type(&ty_tokens.to_string())?;

        (self.item_impl_factory)(ty)
    }
}