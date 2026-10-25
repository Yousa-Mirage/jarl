use crate::diagnostic::{Diagnostic, Fix, Violation};
use crate::rule_set::Rule;
use crate::utils::get_function_name;
use air_r_syntax::*;
use biome_rowan::{AstNode, WalkEvent};

pub struct NegativeIndex;

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for unary minus in `[]` indices, such as `x[-idx]`, except for
/// directly negated literals such as `x[-1]`.
/// A `c()` call is also treated as literal when all its arguments are literals.
///
/// ## Why is this bad?
///
/// If `idx` is empty, `x[-idx]` selects no elements instead of keeping all of
/// `x`. This can happen with `x[-which(condition)]` when nothing matches.
///
/// This rule also reports expressions such as `x[-1 * idx]`, `x[c(-1, -idx)]`,
/// and `x[-2 * 3]`. It does not determine whether an index is actually empty or
/// whether a surrounding condition already checks its length.
///
/// No automatic fix is provided. Handle empty indices explicitly or use a
/// logical index when possible.
///
/// ## Example
///
/// ```r
/// x <- 1:10
/// x[-which(x == 99)]
/// ```
///
/// Use instead:
///
/// ```r
/// x <- 1:10
/// x[x != 99]
/// ```
/// <!-- docs: end -->
impl Violation for NegativeIndex {
    fn rule(&self) -> Rule {
        Rule::NegativeIndex
    }

    fn body(&self) -> String {
        "Non-literal negative indices may produce unexpected results when empty.".to_string()
    }

    fn suggestion(&self) -> Option<String> {
        Some("Handle empty indices explicitly or use a logical index.".to_string())
    }
}

pub fn negative_index(ast: &RSubset) -> anyhow::Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    for arg in ast.arguments()?.items() {
        let Some(index) = arg?.value() else {
            continue;
        };

        let mut expression = index.clone();
        while let AnyRExpression::RParenthesizedExpression(paren) = expression {
            expression = paren.body()?;
        }

        if let AnyRExpression::RUnaryExpression(unary) = expression
            && unary.operator()?.kind() == RSyntaxKind::MINUS
            && is_literal(unary.argument()?)?
        {
            continue;
        }

        if contains_negative(index.syntax())? {
            diagnostics.push(Diagnostic::new(
                NegativeIndex,
                index.syntax().text_trimmed_range(),
                Fix::empty(),
            ));
        }
    }
    Ok(diagnostics)
}

/// Skip nested indices to avoid duplicate reports, but inspect their subset objects.
/// Function bodies do not compute this index.
fn contains_negative(node: &RSyntaxNode) -> anyhow::Result<bool> {
    let mut preorder = node.preorder();
    while let Some(event) = preorder.next() {
        let WalkEvent::Enter(node) = event else {
            continue;
        };
        if let Some(call) = RCall::cast(node.clone())
            && is_literal_c_call(&call)?
        {
            preorder.skip_subtree();
            continue;
        }
        if RSubsetArguments::can_cast(node.kind())
            || RSubset2Arguments::can_cast(node.kind())
            || RFunctionDefinition::can_cast(node.kind())
        {
            preorder.skip_subtree();
        } else if let Some(unary) = RUnaryExpression::cast(node)
            && unary.operator()?.kind() == RSyntaxKind::MINUS
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// A `c()` made only of literals has no runtime-dependent values to warn about.
fn is_literal_c_call(call: &RCall) -> anyhow::Result<bool> {
    if get_function_name(call.function()?) != "c" {
        return Ok(false);
    }
    for arg in call.arguments()?.items() {
        let Some(value) = arg?.value() else {
            return Ok(false);
        };
        if !is_literal(value)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn is_literal(mut expression: AnyRExpression) -> anyhow::Result<bool> {
    while let AnyRExpression::RParenthesizedExpression(paren) = expression {
        expression = paren.body()?;
    }
    match expression {
        AnyRExpression::AnyRValue(
            AnyRValue::RIntegerValue(_)
            | AnyRValue::RDoubleValue(_)
            | AnyRValue::RComplexValue(_)
            | AnyRValue::RStringValue(_),
        )
        | AnyRExpression::RNaExpression(_)
        | AnyRExpression::RNullExpression(_)
        | AnyRExpression::RTrueExpression(_)
        | AnyRExpression::RFalseExpression(_)
        | AnyRExpression::RInfExpression(_)
        | AnyRExpression::RNanExpression(_) => Ok(true),
        AnyRExpression::RCall(call) => is_literal_c_call(&call),
        AnyRExpression::RUnaryExpression(unary) => Ok(matches!(
            unary.operator()?.kind(),
            RSyntaxKind::PLUS | RSyntaxKind::MINUS
        ) && is_literal(unary.argument()?)?),
        _ => Ok(false),
    }
}
