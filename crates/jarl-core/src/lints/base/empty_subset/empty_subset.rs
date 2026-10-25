use crate::diagnostic::{Diagnostic, Fix, ViolationData};
use crate::rule_set::Rule;
use crate::utils::get_arg_by_position;
use air_r_syntax::*;
use biome_rowan::{AstNode, AstSeparatedList};

/// <!-- docs: start -->
/// Version added: 0.8.0
///
/// ## What it does
///
/// Checks for empty `[]` and `[[]]` indices.
/// Empty `[]` used in replacement assignments, such as `x[] <- 1`, is allowed.
///
/// ## Why is this bad?
///
/// `x[]` usually selects all elements of `x`, so the brackets are unnecessary.
/// `x[[]]` normally raises an error because no subscript is supplied.
/// Indices containing commas, such as `x[,]`, are not checked.
///
/// This rule has no automatic fix because custom subsetting methods can change
/// the behavior of `x[]` and `x[[]]`.
///
/// ## Example
///
/// ```r
/// x[]
/// x[[]]
/// ```
///
/// Use instead:
///
/// ```r
/// x
/// x[[1]]
/// ```
/// <!-- docs: end -->
pub fn empty_subset(ast: &RSubset) -> anyhow::Result<Option<Diagnostic>> {
    let args = ast.arguments()?;
    // A missing closing bracket is a parse error, not an empty subset.
    if args.r_brack_token().is_err()
        || !args.items().is_empty()
        || is_replacement_target(ast.syntax())?
    {
        return Ok(None);
    }

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::EmptySubset,
            "Empty `[]` usually selects the entire object.".to_string(),
            Some("Remove the brackets if selecting the entire object is intended.".to_string()),
        ),
        args.syntax().text_trimmed_range(),
        Fix::empty(),
    )))
}

pub fn empty_subset2(ast: &RSubset2) -> anyhow::Result<Option<Diagnostic>> {
    let args = ast.arguments()?;
    if args.r_brack2_token().is_err() || !args.items().is_empty() {
        return Ok(None);
    }

    Ok(Some(Diagnostic::new(
        ViolationData::new(
            Rule::EmptySubset,
            "Empty `[[]]` normally raises a missing subscript error.".to_string(),
            Some("Supply an index to select an element.".to_string()),
        ),
        args.syntax().text_trimmed_range(),
        Fix::empty(),
    )))
}

/// Whether this `x[]` lies on the object path modified by a replacement
/// assignment. A true result suppresses the lint because removing `[]` from
/// that path can change the assignment's behavior.
///
/// Follow enclosing expressions until an assignment confirms that the path is
/// its target, as in `names(x[]) <- value`. Stop when the expression is used
/// only to compute a value, as with the index `y[]` in `x[y[]] <- value`.
fn is_replacement_target(node: &RSyntaxNode) -> anyhow::Result<bool> {
    let mut current = node.clone();
    for parent in node.ancestors().skip(1).filter_map(AnyRExpression::cast) {
        let is_target = match &parent {
            // Parentheses preserve the role of the enclosed expression.
            AnyRExpression::RParenthesizedExpression(_) => true,

            // Continue only through the object being subsetted or accessed.
            // Indices and member names do not identify the object to modify.
            AnyRExpression::RSubset(subset) => subset.function()?.syntax() == &current,
            AnyRExpression::RSubset2(subset) => subset.function()?.syntax() == &current,
            AnyRExpression::RExtractExpression(extract) => extract.left()?.syntax() == &current,

            // A replacement function modifies its first argument; subsequent
            // arguments, as in `f(x, y[]) <- value`, are ordinary reads.
            AnyRExpression::RCall(call) => get_arg_by_position(&call.arguments()?.items(), 1)
                .and_then(|arg| arg.value())
                .is_some_and(|value| value.syntax() == &current),

            // An assignment confirms the path only if it is the assignment
            // target. Account for both leftward and rightward assignments.
            AnyRExpression::RBinaryExpression(binary) => {
                let target = match binary.operator()?.kind() {
                    RSyntaxKind::ASSIGN | RSyntaxKind::SUPER_ASSIGN | RSyntaxKind::EQUAL => {
                        binary.left()?
                    }
                    RSyntaxKind::ASSIGN_RIGHT | RSyntaxKind::SUPER_ASSIGN_RIGHT => {
                        binary.right()?
                    }
                    _ => return Ok(false),
                };
                return Ok(target.syntax() == &current);
            }

            // Other expressions compute values rather than extend the path
            // to the object being modified, so stop following them.
            _ => false,
        };
        if !is_target {
            return Ok(false);
        }
        current = parent.into_syntax();
    }
    Ok(false)
}
