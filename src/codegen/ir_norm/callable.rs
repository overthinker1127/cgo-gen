use crate::parser::{CppFunction, CppMethod, CppOperator};

#[derive(Clone, Copy)]
pub(super) enum CppCallableRef<'a> {
    Function(&'a CppFunction),
    Method(&'a CppMethod),
    Operator(&'a CppOperator),
}

#[derive(Clone, Copy)]
pub(super) struct CppReturnSignature<'a> {
    pub(super) ty: &'a str,
    pub(super) canonical_ty: &'a str,
    pub(super) is_function_pointer: bool,
}

impl<'a> CppCallableRef<'a> {
    pub(super) fn return_signature(self) -> CppReturnSignature<'a> {
        match self {
            Self::Function(function) => CppReturnSignature {
                ty: &function.return_type,
                canonical_ty: &function.return_canonical_type,
                is_function_pointer: function.return_is_function_pointer,
            },
            Self::Method(method) => CppReturnSignature {
                ty: &method.return_type,
                canonical_ty: &method.return_canonical_type,
                is_function_pointer: method.return_is_function_pointer,
            },
            Self::Operator(operator) => CppReturnSignature {
                ty: &operator.return_type,
                canonical_ty: &operator.return_canonical_type,
                is_function_pointer: operator.return_is_function_pointer,
            },
        }
    }
}
