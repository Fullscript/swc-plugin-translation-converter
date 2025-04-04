use swc_core::common::SyntaxContext;
use swc_ecma_ast::{Expr, Ident, MemberExpr};

#[derive(Clone)]
pub struct ExprWithComputed {
    pub expr: Box<Expr>,
    pub computed: bool,
}

/// Recurses through a MemberExpression that contains an l translation object and collects all Identifiers
///
/// # Examples
/// ```
/// let member_expr = l.common.foobar; // (as a MemberExpression from AST)
/// let translation_value = serializers::l::expr(member_expr);
///
/// assert_eq!("common:foobar", translation_value);
/// ```
///
/// ```
/// let bar = "baz";
/// let member_expr = l.common.foo[bar];
/// let translation_value = serializers::l::expr(member_expr);
///
/// assert_eq!(`common:foo.${bar}`, vec![
///     ComputerOrIdent { ident: common_ident, computed: false },
///     ComputerOrIdent { ident: foo_ident, computed: false },
///     ComputerOrIdent { ident: bar_ident, computed: true }
/// ]);
/// ```
pub fn member_expr(
    member: &MemberExpr,
    exprs: &mut Vec<ExprWithComputed>,
) -> Vec<ExprWithComputed> {
    // Case where member_expr is a nested collection of MemberExpr
    // We need to recursively continue down the AST collecting all Ident as we go
    // ex: l.common.foobar
    if member.obj.is_member() {
        member_expr(member.obj.as_member().unwrap(), exprs);
    }

    // If prop is an Ident, add it to the list of exprs to convert into a StringLiteral
    if member.prop.is_ident() {
        let ident_name = member.prop.as_ident().unwrap();
        let ident = Ident::new(
            ident_name.sym.clone(),
            ident_name.span,
            SyntaxContext::empty(),
        );
        exprs.push(ExprWithComputed {
            expr: Box::new(Expr::Ident(ident)),
            computed: false,
        });
    // If prop is a computed expr we need to convert [bar] into ${bar} before adding to the list of identifiers
    // ex: l.common.foo[bar]
    } else if member.prop.is_computed() {
        let computed = member.prop.as_computed().unwrap();

        exprs.push(ExprWithComputed {
            expr: computed.expr.clone(),
            computed: true,
        });
    }

    return exprs.clone();
}

/// Joins a list of identifiers with the specified delimiter and outputs a string
/// Assumes that exprs contains only identifiers
fn join_identifiers(exprs: &[ExprWithComputed], delimiter: &str) -> String {
    return exprs
        .iter()
        .map(|e| e.expr.as_ident().unwrap().sym.to_string())
        .collect::<Vec<String>>()
        .join(delimiter);
}

/// Concatenates a list of identifier values (String) into a single String
///
/// # Examples
/// ```
/// let identifiers = vec!["common", "foobar"];
/// let translation = concatenate_identifiers(identifiers);
///
/// assert_eq!("common:foobar", translation);
/// ```
///
/// ```
/// let identifiers = vec!["common", "foo1", "foo2"];
/// let translation = concatenate_identifiers(identifiers);
///
/// assert_eq!("common:foo1.foo2", translation);
/// ```
pub fn concatenate_identifiers(identifiers: Vec<ExprWithComputed>) -> String {
    // If collected identifiers is size 2 or more, we can safely concatenate the contents of identifiers
    // ex: ["common", "foo1", "foo2"]
    if identifiers.len() >= 2 {
        // The first element in our vector is the translation namespace, second element is the element following the namespace
        // ex: ["common", "foo1", "foo2"] from l.common.foo1.foo2
        // "common" and "foo1" need to be joined by a ":" where everything else (properties) is joined by a "."
        // The following statement becomes
        //  - namespace_group = ["common", "foo1"]
        //  - properties = ["foo2"]
        let (namespace_group, properties) = identifiers.split_at(2);

        // Case where identifiers was just ["common", "foobar"] originally
        // - namespace_group = ["common", "foobar"]
        // - properties = []
        if properties.is_empty() {
            return join_identifiers(namespace_group, ":");
        } else {
            // Case where identifiers was ["common", "foo1", "foo2", ...] originally
            // - namespace_group = ["common", "foo1"]
            // - properties = ["foo2", ...]
            return format!(
                "{}.{}",
                join_identifiers(namespace_group, ":"),
                join_identifiers(properties, "."),
            );
        }
    } else {
        return "".to_string();
    }
}
