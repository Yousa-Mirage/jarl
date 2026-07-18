use crate::checker::Checker;
use crate::diagnostic::*;
use crate::utils::{
    get_arg_by_name_then_position, get_function_name, get_function_namespace_prefix,
    node_contains_comments,
};
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList, Direction};

/// Version added: 0.6.0
///
/// ## What it does
///
/// Checks for indirect regex subsetting such as `x[grep(pattern, x)]`,
/// `x[grepl(pattern, x)]`, and their stringr equivalents.
///
/// ## Why is this bad?
///
/// Functions such as `grepv()` and `stringr::str_subset()` express the intent
/// to return matching values directly instead of spelling out a two-step
/// match-and-subset operation.
///
/// This rule has an unsafe automatic fix because the replacement can change
/// the result. For example, subsetting a factor returns a factor, while
/// `grepv()` returns a character vector. `str_subset()` also drops missing
/// values that a logical subset would retain.
///
/// ## Example
///
/// ```r
/// x[grep(pattern, x)]
/// x[stringr::str_detect(x, pattern)]
/// ```
///
/// Use instead:
/// ```r
/// grepv(pattern, x)
/// stringr::str_subset(x, pattern)
/// ```
///
/// For code supporting R versions before 4.5.0, the base R replacement uses
/// `grep(pattern, x, value = TRUE)` instead of `grepv()`.
///
/// ## References
///
/// See `?grepv` and `?stringr::str_subset`
pub fn regex_subset(ast: &RSubset, checker: &Checker) -> anyhow::Result<Option<Diagnostic>> {
    if is_assignment_target(ast) {
        return Ok(None);
    }

    let RSubsetFields { function, arguments } = ast.as_fields();
    let subsetted = function?;
    let arguments = arguments?.items();

    if arguments.iter().count() != 1 {
        return Ok(None);
    }

    let index = unwrap_or_return_none!(arguments.iter().next()).clone()?;
    if index.name_clause().is_some() {
        return Ok(None);
    }

    let index = unwrap_or_return_none!(index.value());
    let call = unwrap_or_return_none!(index.as_r_call());
    let call_function = call.function()?;
    let function_name = get_function_name(call_function.clone());
    let call_arguments = call.arguments()?.items();

    let (searched, is_base) = match function_name.as_str() {
        "grep" | "grepl" => (
            unwrap_or_return_none!(get_arg_by_name_then_position(&call_arguments, "x", 2)),
            true,
        ),
        "str_detect" | "str_which" => (
            unwrap_or_return_none!(get_arg_by_name_then_position(&call_arguments, "string", 1)),
            false,
        ),
        _ => return Ok(None),
    };
    let searched = unwrap_or_return_none!(searched.value());

    if !expressions_match(&subsetted, &searched) {
        return Ok(None);
    }

    let namespace = get_function_namespace_prefix(call_function).unwrap_or_default();
    let replacement = if is_base {
        let use_grepv = checker
            .minimum_r_version
            .is_some_and(|version| version >= (4, 5, 0));
        let arguments = base_replacement_arguments(&call_arguments, &function_name, use_grepv)?;
        let replacement_function = if use_grepv { "grepv" } else { "grep" };
        format!("{namespace}{replacement_function}({arguments})")
    } else {
        let arguments = call_arguments
            .iter()
            .map(|argument| Ok(argument?.to_trimmed_text()))
            .collect::<anyhow::Result<Vec<_>>>()?
            .join(", ");
        format!("{namespace}str_subset({arguments})")
    };

    let range = ast.syntax().text_trimmed_range();
    let linted = ast.to_trimmed_text();

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            "regex_subset".to_string(),
            format!("`{replacement}` is more direct than `{linted}`."),
            Some(format!("Use `{replacement}` instead.")),
        ),
        range,
        Fix {
            content: replacement,
            start: range.start().into(),
            end: range.end().into(),
            to_skip: node_contains_comments(ast.syntax()),
        },
    )))
}

fn base_replacement_arguments(
    arguments: &RArgumentList,
    function_name: &str,
    use_grepv: bool,
) -> anyhow::Result<String> {
    let value_argument = (function_name == "grep")
        .then(|| get_arg_by_name_then_position(arguments, "value", 5))
        .flatten();

    let grepl_has_shifted_positional_argument = function_name == "grepl"
        && arguments.iter().enumerate().any(|(index, argument)| {
            index >= 4
                && argument
                    .ok()
                    .is_some_and(|argument| argument.name_clause().is_none())
        });

    let mut replacement_arguments = Vec::new();
    for argument in arguments.iter() {
        let argument = argument?;
        if value_argument
            .as_ref()
            .is_some_and(|value| value.syntax() == argument.syntax())
        {
            replacement_arguments.push("value = TRUE".to_string());
        } else {
            replacement_arguments.push(argument.to_trimmed_text().to_string());
        }
    }

    let needs_value_argument =
        !use_grepv || value_argument.is_some() || grepl_has_shifted_positional_argument;
    if needs_value_argument && value_argument.is_none() {
        replacement_arguments.push("value = TRUE".to_string());
    }

    Ok(replacement_arguments.join(", "))
}

fn expressions_match(left: &AnyRExpression, right: &AnyRExpression) -> bool {
    let tokens = |expression: &AnyRExpression| {
        expression
            .syntax()
            .descendants_tokens(Direction::Next)
            .map(|token| (token.kind(), token.text_trimmed().to_string()))
            .collect::<Vec<_>>()
    };

    tokens(left) == tokens(right)
}

fn is_assignment_target(ast: &RSubset) -> bool {
    let Some(parent) = ast.syntax().parent().and_then(RBinaryExpression::cast) else {
        return false;
    };
    let RBinaryExpressionFields { left, operator, right } = parent.as_fields();
    let (Ok(left), Ok(operator), Ok(right)) = (left, operator, right) else {
        return false;
    };

    match operator.kind() {
        RSyntaxKind::EQUAL | RSyntaxKind::ASSIGN => left.syntax() == ast.syntax(),
        RSyntaxKind::ASSIGN_RIGHT => right.syntax() == ast.syntax(),
        _ => false,
    }
}
