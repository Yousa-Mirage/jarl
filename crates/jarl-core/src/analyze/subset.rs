use crate::checker::Checker;
use crate::rule_set::Rule;
use air_r_syntax::RSubset;

use crate::lints::base::regex_subset::regex_subset::regex_subset;
use crate::lints::base::sort::sort::sort;

pub fn subset(r_expr: &RSubset, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::RegexSubset) {
        checker.report_diagnostic(regex_subset(r_expr, checker)?);
    }
    if checker.is_rule_enabled(Rule::Sort) {
        checker.report_diagnostic(sort(r_expr)?);
    }
    Ok(())
}
