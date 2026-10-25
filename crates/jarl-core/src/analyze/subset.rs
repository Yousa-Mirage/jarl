use crate::checker::Checker;
use crate::rule_set::Rule;
use air_r_syntax::{RSubset, RSubset2};

use crate::lints::base::empty_subset::empty_subset::{empty_subset, empty_subset2};
use crate::lints::base::sort::sort::sort;

pub fn subset(r_expr: &RSubset, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::EmptySubset) {
        checker.report_diagnostic(empty_subset(r_expr)?);
    }
    if checker.is_rule_enabled(Rule::Sort) {
        checker.report_diagnostic(sort(r_expr)?);
    }
    Ok(())
}

pub fn subset2(r_expr: &RSubset2, checker: &mut Checker) -> anyhow::Result<()> {
    if checker.is_rule_enabled(Rule::EmptySubset) {
        checker.report_diagnostic(empty_subset2(r_expr)?);
    }
    Ok(())
}
