use proc_macro2::Ident;
use syn::{DeriveInput, ExprClosure, Generics, Path, Token, Type, TypeGroup, Visibility};
use syn::punctuated::Punctuated;

pub struct AutumnModuleTS {
    pub services: Vec<AutumnServiceTS>,
    pub functions: Vec<AutumnFunctionTS>,
}

pub struct AutumnServiceTS {
    pub source: DeriveInput,
    pub initialize: Vec<AutumnFunctionTS>,
    pub functions: Vec<AutumnFunctionTS>,
}

pub struct AutumnFunctionTS {
    pub visibility: Visibility,
    pub fn_keyword: Token![fn],
    pub ident: Ident,
    pub generics: Generics,
    pub arguments: Punctuated<AutumnFunctionArgumentTS, Token![,]>,
    pub return_type: Option<(Token![->], Type)>,
    pub closure: ExprClosure,
}

pub struct AutumnFunctionArgumentTS {
    pub mut_keyword: Option<Token![mut]>,
    pub ident: Ident,
    pub double_dot: Option<Token![:]>,
    pub ty: Type,
}