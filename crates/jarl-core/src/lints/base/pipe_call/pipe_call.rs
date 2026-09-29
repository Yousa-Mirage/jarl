use crate::diagnostic::*;
use crate::rule_set::Rule;
use air_r_syntax::*;
use biome_rowan::AstNode;

pub fn pipe_call(ast: &RBinaryExpression) -> anyhow::Result<Option<Diagnostic>> {
    let RBinaryExpressionFields { left: _, operator, right } = ast.as_fields();
    let operator = operator?;

    // `%$%` exposes names rather than applying a function.
    if operator.kind() != RSyntaxKind::SPECIAL
        || !matches!(operator.text_trimmed(), "%>%" | "%!>%" | "%T>%" | "%<>%")
    {
        return Ok(None);
    }

    // Check if the right-hand side is an identifier (e.g., `x`),
    // rather than a function call (e.g., `x()`).
    let AnyRExpression::RIdentifier(right) = right? else {
        return Ok(None);
    };
    let name = right.to_trimmed_string();
    let range = right.syntax().text_trimmed_range();
    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::PipeCall,
            "Use an explicit call on the right-hand side of a magrittr pipe.".to_string(),
            Some(format!("Use `{name}()` instead.")),
        ),
        range,
        Fix::new(range, format!("{name}()"), false),
    )))
}
