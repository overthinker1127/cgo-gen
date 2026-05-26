use anyhow::Result;

use crate::{
    domain::kind::{IrFunctionKind, IrTypeKind},
    parser::{CppOperator, CppOperatorToken, CppParam},
};

use super::{
    IrFunction, IrOperator, IrParam, IrType, NormalizeEnv, callable::CppCallableRef,
    callable_skip_reason, cpp_qualified, normalize_callable_return, normalize_cpp_params,
    push_skipped_declaration, symbol_name,
};

pub(super) fn normalize_operator(
    env: &mut NormalizeEnv<'_>,
    owner: Option<(&crate::parser::CppRecord, &str)>,
    operator: &CppOperator,
    cpp_params: &[CppParam],
) -> Result<Option<IrFunction>> {
    let cpp_name = cpp_name(operator);
    if operator_generation_unsupported(&operator.token) {
        push_skipped_declaration(
            env,
            cpp_name,
            format!(
                "operator `{}` is unsupported for wrapper generation in v1",
                operator.spelling
            ),
        );
        return Ok(None);
    }
    if let Some(reason) = callable_skip_reason(CppCallableRef::Operator(operator), cpp_params, env)
    {
        push_skipped_declaration(env, cpp_name, reason);
        return Ok(None);
    }

    let tail = operator_name_tail(&operator.token);
    let mut params = Vec::new();
    let (kind, method_of, owner_cpp_type, name_owner) = if let Some((record, handle_name)) = owner {
        let qualified = cpp_qualified(&record.namespace, &record.name);
        params.push(IrParam {
            name: "self".to_string(),
            ty: IrType {
                kind: IrTypeKind::Opaque,
                cpp_type: if operator.is_const {
                    format!("const {}*", qualified)
                } else {
                    format!("{}*", qualified)
                },
                c_type: if operator.is_const {
                    format!("const {handle_name}*")
                } else {
                    format!("{handle_name}*")
                },
                handle: Some(handle_name.to_string()),
            },
        });
        (
            IrFunctionKind::Method,
            Some(handle_name.to_string()),
            Some(qualified),
            record.name.as_str(),
        )
    } else {
        (IrFunctionKind::Function, None, None, "")
    };

    params.extend(normalize_cpp_params(env, cpp_params)?);

    Ok(Some(IrFunction {
        name: symbol_name(env.config, &operator.namespace, name_owner, &tail),
        kind,
        cpp_name,
        method_of,
        owner_cpp_type,
        is_const: owner.map(|_| operator.is_const),
        field_accessor: None,
        operator: Some(IrOperator {
            spelling: operator.spelling.clone(),
            token: operator.token.clone(),
        }),
        returns: normalize_callable_return(env, CppCallableRef::Operator(operator))?,
        params,
    }))
}

fn operator_generation_unsupported(token: &CppOperatorToken) -> bool {
    matches!(token, CppOperatorToken::Unsupported(_))
}

fn cpp_name(operator: &CppOperator) -> String {
    match &operator.owner {
        Some(owner) => format!("{owner}::{}", operator.spelling),
        None => cpp_qualified(&operator.namespace, &operator.spelling),
    }
}

pub(crate) fn operator_name_tail(token: &CppOperatorToken) -> String {
    match token {
        CppOperatorToken::Plus => "OperPlus".to_string(),
        CppOperatorToken::Minus => "OperMinus".to_string(),
        CppOperatorToken::Multiply => "OperMultiply".to_string(),
        CppOperatorToken::Divide => "OperDivide".to_string(),
        CppOperatorToken::Modulo => "OperModulo".to_string(),
        CppOperatorToken::Equal => "OperEqual".to_string(),
        CppOperatorToken::Assign => "OperAssign".to_string(),
        CppOperatorToken::PlusAssign => "OperPlusAssign".to_string(),
        CppOperatorToken::MinusAssign => "OperMinusAssign".to_string(),
        CppOperatorToken::MultiplyAssign => "OperMultiplyAssign".to_string(),
        CppOperatorToken::DivideAssign => "OperDivideAssign".to_string(),
        CppOperatorToken::ModuloAssign => "OperModuloAssign".to_string(),
        CppOperatorToken::NotEqual => "OperNotEqual".to_string(),
        CppOperatorToken::Less => "OperLess".to_string(),
        CppOperatorToken::LessEq => "OperLessEq".to_string(),
        CppOperatorToken::Greater => "OperGreater".to_string(),
        CppOperatorToken::GreaterEq => "OperGreaterEq".to_string(),
        CppOperatorToken::Spaceship => "OperSpaceship".to_string(),
        CppOperatorToken::Amp => "OperAmp".to_string(),
        CppOperatorToken::Pipe => "OperPipe".to_string(),
        CppOperatorToken::Caret => "OperCaret".to_string(),
        CppOperatorToken::Tilde => "OperTilde".to_string(),
        CppOperatorToken::Not => "OperNot".to_string(),
        CppOperatorToken::AmpAssign => "OperAmpAssign".to_string(),
        CppOperatorToken::PipeAssign => "OperPipeAssign".to_string(),
        CppOperatorToken::CaretAssign => "OperCaretAssign".to_string(),
        CppOperatorToken::LessLess => "OperLessLess".to_string(),
        CppOperatorToken::GreaterGreater => "OperGreaterGreater".to_string(),
        CppOperatorToken::LessLessAssign => "OperLessLessAssign".to_string(),
        CppOperatorToken::GreaterGreaterAssign => "OperGreaterGreaterAssign".to_string(),
        CppOperatorToken::AndAnd => "OperAndAnd".to_string(),
        CppOperatorToken::OrOr => "OperOrOr".to_string(),
        CppOperatorToken::Comma => "OperComma".to_string(),
        CppOperatorToken::Arrow => "OperArrow".to_string(),
        CppOperatorToken::ArrowStar => "OperArrowStar".to_string(),
        CppOperatorToken::Array => "OperArray".to_string(),
        CppOperatorToken::Func => "OperFunc".to_string(),
        CppOperatorToken::Increment => "OperIncrement".to_string(),
        CppOperatorToken::Decrement => "OperDecrement".to_string(),
        CppOperatorToken::Conversion(target) => {
            format!("Oper{}", pascal_identifier(target))
        }
        CppOperatorToken::Unsupported(token) => {
            format!("Oper{}", pascal_identifier(token))
        }
    }
}

fn pascal_identifier(value: &str) -> String {
    let mut out = String::new();
    let mut capitalize = true;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            if capitalize {
                out.push(ch.to_ascii_uppercase());
                capitalize = false;
            } else {
                out.push(ch);
            }
        } else {
            capitalize = true;
        }
    }
    if out.is_empty() {
        "Unsupported".to_string()
    } else {
        out
    }
}
