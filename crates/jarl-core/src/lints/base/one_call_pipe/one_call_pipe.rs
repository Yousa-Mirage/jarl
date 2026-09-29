use crate::diagnostic::*;
use crate::rule_set::Rule;
use crate::utils::node_contains_comments;
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// <!-- docs: start -->
/// Version added: 0.7.0
///
/// ## What it does
///
/// Reports pipes whose expression contains only one function call. This applies
/// to both the native pipe (`|>`) and magrittr pipes (`%>%`, `%!>%`, `%T>%`,
/// and `%<>%`). This rule is disabled by default.
///
/// Calls anywhere in the piped expression count, including calls on the left
/// side. An expression on the left side without a call, such as `x + 1`, does
/// not add to the count; an expression containing a call, such as `f(x) + 1`,
/// does.
///
/// ## Why is this bad?
///
/// A pipe with only one call is often clearer as a regular function call.
///
/// A safe automatic fix is available for `|>` and `%>%` when the right-hand
/// side is a call with no arguments and the pipe expression contains no
/// comments. Other magrittr pipe variants have different semantics and are
/// reported without a fix.
///
/// ## Example
///
/// ```r
/// x |> sum()
/// 1:3 %>% mean()
/// ```
///
/// Use instead:
///
/// ```r
/// sum(x)
/// mean(1:3)
/// ```
///
/// A pipe with multiple calls is not reported, for example:
///
/// ```r
/// rowSums(x) %>% mean()
/// ```
/// <!-- docs: end -->
pub fn one_call_pipe(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left, operator, right } = ast.as_fields();
    let operator = operator?;
    let is_pipe = operator.kind() == RSyntaxKind::PIPE
        || (operator.kind() == RSyntaxKind::SPECIAL
            && matches!(operator.text_trimmed(), "%>%" | "%!>%" | "%T>%" | "%<>%"));
    if !is_pipe {
        return Ok(None);
    }

    // Only inspect the outermost pipe in a chain. Its subtree includes all
    // calls from earlier pipe stages, avoiding a false positive on an
    // intermediate stage of a multi-call pipeline.
    if ast
        .syntax()
        .ancestors()
        .skip(1)
        .filter_map(RBinaryExpression::cast)
        .any(|ancestor| {
            ancestor.operator().is_ok_and(|operator| {
                operator.kind() == RSyntaxKind::PIPE
                    || (operator.kind() == RSyntaxKind::SPECIAL
                        && matches!(operator.text_trimmed(), "%>%" | "%!>%" | "%T>%" | "%<>%"))
            })
        })
    {
        return Ok(None);
    }

    // Count function calls across the whole pipe expression. In particular,
    // calls nested in the left expression count: `f(x) + 1 |> abs()` has two.
    let calls = ast.syntax().descendants().filter_map(RCall::cast).count();
    if calls != 1 {
        return Ok(None);
    }

    let range = ast.syntax().text_trimmed_range();
    // Report all recognized pipes, but only fix `|>` and `%>%`; the other
    // variants have extra semantics that a plain call would lose.
    let fixable_pipe = operator.kind() == RSyntaxKind::PIPE
        || (operator.kind() == RSyntaxKind::SPECIAL && operator.text_trimmed() == "%>%");
    let replacement = pipe_replacement(ast, left?, right?, fixable_pipe)?;

    let fix = match &replacement {
        Some((replacement, true)) => Fix::new(range, replacement.clone(), false),
        _ => Fix::empty(),
    };
    let suggestion = Some(match replacement {
        Some((replacement, _)) => format!("Replace with `{replacement}`."),
        None => "Use a regular function call instead.".to_string(),
    });
    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::OneCallPipe,
            "Avoid using a pipe for an expression with only one function call.".to_string(),
            suggestion,
        ),
        range,
        fix,
    )))
}

fn pipe_replacement(
    ast: &RBinaryExpression,
    left: AnyRExpression,
    right: AnyRExpression,
    fixable_pipe: bool,
) -> anyhow::Result<Option<(String, bool)>> {
    // Don't suggest a fix that either changes special-pipe semantics or
    // discards comments from the expression.
    if !fixable_pipe || node_contains_comments(ast.syntax()) {
        return Ok(None);
    }

    let AnyRExpression::RCall(call) = right else {
        return Ok(None);
    };
    let has_placeholder = call
        .syntax()
        .descendants()
        .filter_map(RIdentifier::cast)
        .any(|identifier| {
            identifier
                .name_token()
                .is_ok_and(|token| matches!(token.text_trimmed(), "." | "_"))
        });
    // Moving the left operand into the call would change how pipe placeholders
    // are interpreted, so leave these cases to the generic suggestion.
    if has_placeholder {
        return Ok(None);
    }

    let function = call.function()?;
    let function_text = function.syntax().text_trimmed().to_string();
    let call_text = call.syntax().text_trimmed().to_string();
    // Preserve the RHS arguments verbatim while preparing a concrete suggestion.
    let Some(args_text) = call_text.strip_prefix(&function_text).and_then(|suffix| {
        let open = suffix.find('(')?;
        let close = suffix.rfind(')')?;
        (open < close).then(|| suffix[open + 1..close].trim().to_string())
    }) else {
        return Ok(None);
    };

    let replacement = if args_text.is_empty() {
        format!("{function_text}({})", left.syntax().text_trimmed())
    } else {
        format!(
            "{function_text}({}, {args_text})",
            left.syntax().text_trimmed()
        )
    };
    // A suggestion can include existing arguments, but auto-fix only the simple
    // zero-argument RHS where the rewrite is straightforward.
    let can_fix = call.arguments()?.items().is_empty();
    Ok(Some((replacement, can_fix)))
}
