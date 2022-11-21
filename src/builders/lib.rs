use swc_core::{
    common::Span,
    ecma::ast::{Expr, JSXExpr, Lit, MemberExpr, Str, Tpl, TplElement},
};

use crate::builders::{serializers, utils};

use super::serializers::ComputedOrIdent;

/// Generates a Box<Expr> give a MemberExpr and Span
///
/// # Examples
/// ```
/// let span = Span::new(BytePos(0), BytePos(0), SyntaxContext::empty()).into();
/// let foobar_ident = Ident::new("foobar".into(), span).into();
/// let l_ident = Ident::new("l".into(), span).into();
/// let common_ident = Ident::new("common".into(), span).into();
///
/// let nested_member_expr = MemberExpr {
///     obj: Box::new(common_ident),
///     prop: MemberProp::Ident(l_ident),
///     span: span,
/// };
///
/// // A member representing l.common.foobar
/// let member_expr = MemberExpr {
///     obj: Box::new(nested_member_expr.into()),
///     prop: MemberProp::Ident(foobar_ident),
///     span: span,
/// };
///
/// assert_eq!(
///     box_expr(member_expr, span),
///     Box::new(Expr::Lit(Lit::Str(Str {
///         raw: Some(r#""common:foobar""#),
///         value: "common:foobar",
///         span: span,
///     })))
/// );
/// ```
pub fn box_expr(member: &MemberExpr, span: Span) -> Option<Box<Expr>> {
    // if member doesn't contain an l object no need to do anything
    if !utils::has_child_l(member) {
        return None;
    }

    // Serializes all Ident in member into a single String l.common.foobar -> "common:foobar"
    let identifiers = serializers::member_expr(member, &mut vec![]);

    // If builders::serializers::l::expr could not generate a String representation of Member it returns ""
    // This means that the translation l.common is invalid
    if identifiers.is_empty() {
        return None;
    }

    // identifiers contains a computed Ident we need to generate an Expr::Tpl
    if identifiers.iter().any(|ci| ci.computed) {
        let template_expr = expr_tpl(identifiers, span);
        return Some(Box::new(template_expr));
    }

    // translation_value does not contain an interpolated value so we generate a Expr::Lit
    let expr = expr_lit(identifiers, span);

    // This Expr can then be inserted into the AST to complete the code transformation process
    return Some(Box::new(expr));
}

/// Generates a JSXExpr give a MemberExpr and Span
///
/// # Examples
/// ```
/// let span = Span::new(BytePos(0), BytePos(0), SyntaxContext::empty()).into();
/// let foobar_ident = Ident::new("foobar".into(), span).into();
/// let l_ident = Ident::new("l".into(), span).into();
/// let common_ident = Ident::new("common".into(), span).into();
///
/// let nested_member_expr = MemberExpr {
///     obj: Box::new(common_ident),
///     prop: MemberProp::Ident(l_ident),
///     span: span,
/// };
///
/// // A member representing l.common.foobar
/// let member_expr = MemberExpr {
///     obj: Box::new(nested_member_expr.into()),
///     prop: MemberProp::Ident(foobar_ident),
///     span: span,
/// };
///
/// assert_eq!(
///     jsx_expr(member_expr, span),
///     JSXExpr::Expr(
///         Box::new(Expr::Lit(Lit::Str(Str {
///             raw: Some(r#""common:foobar""#),
///             value: "common:foobar",
///             span: span,
///         })))
///     )
/// );
/// ```
pub fn jsx_expr(member: &MemberExpr, span: Span) -> Option<JSXExpr> {
    let expr = box_expr(member, span);

    if expr.is_none() {
        return None;
    }

    return Some(JSXExpr::Expr(expr.unwrap()));
}

/// Given a String like "common:foobar" expr_lit will generate an Expr::Lit enum
/// We can later inject it into the AST to replace the respective l.common...
///
/// # Examples
/// ```
/// let span = Span::new(BytePos(0), BytePos(0), SyntaxContext::empty()).into();
/// let translation_value = expr("common:foobar", span);
/// let expr = expr(translation_value, span);
///
/// assert_eq!(expr, Expr::Lit(Lit::Str(Str {
///   raw: Some(r#""common:foobar""#),
///   value: "common:foobar",
///   span: span,
/// })));
/// ```
fn expr_lit(identifiers: Vec<ComputedOrIdent>, span: Span) -> Expr {
    let translation_value = serializers::concatenate_identifiers(identifiers);
    // Else condition where translation_value does not contain ${} interpolated values
    // raw properties of a Str need to contain escaped quotations such that they are represented as such in the AST
    // "\"common:foobar\"", this is why we are using r#, SUPER IMPORTANT!
    let translation_raw = format!(r#""{}""#, translation_value);

    let string_literal = Str {
        raw: Some(translation_raw.into()),
        value: translation_value.into(),
        span: span,
    };

    return Expr::Lit(Lit::Str(string_literal));
}

/// Given a String with interpolated values "common:foo${bar}" expr_tpl will generate an Expr::Tpl enum
/// We can later inject it into the AST to replace the respective l.common.foo[bar]
fn expr_tpl(identifiers: Vec<ComputedOrIdent>, span: Span) -> Expr {
    let mut quasis: Vec<TplElement> = vec![];
    let mut quasis_group: String = "".to_string();
    let mut exprs: Vec<Box<Expr>> = vec![];

    for (i, ci) in identifiers.iter().enumerate() {
        let first_iteration = i == 0;
        let last_iteration = i == identifiers.len() - 1;

        // expr that needs to be added to exprs, ex: ${common}
        if ci.computed {
            // if quasis_group is not empty it means that we have prevously collected quasis that needs to be committed to the AST
            if !quasis_group.is_empty() {
                quasis.push(TplElement {
                    span: span,
                    tail: i == identifiers.len() - 1, // if this is the last ident, this quasis needs "tail: true"
                    cooked: Some(quasis_group.clone().into()),
                    raw: quasis_group.clone().into(),
                });

                quasis_group = "".to_string();
            }

            // Each expression within a TemplateLiteral must follow with a . unless it's the namespace
            if !last_iteration && !first_iteration {
                quasis_group = quasis_group + ".";
            }

            if first_iteration {
                // We need to add : as it follows all namespaces computed or not
                quasis_group = quasis_group + ":";

                // If the first element is computed, we must append an empty quasis
                quasis.push(TplElement {
                    span: span,
                    tail: false,
                    cooked: Some("".into()),
                    raw: "".into(),
                });
            }

            // For the above described example, expr would be "bar" after stripping ${}
            exprs.push(Box::new(Expr::Ident(ci.ident.clone())));

            // The last entry within a TemplateLiteral is computed, we need to add an empty trailing quasis
            if i == identifiers.len() - 1 {
                quasis.push(TplElement {
                    span: span,
                    tail: true,
                    cooked: Some("".into()),
                    raw: "".into(),
                });
            }
        } else {
            // not computed ident
            quasis_group = quasis_group + &ci.ident.sym as &str;

            if first_iteration {
                // we can assume that this is the namespace that is being added
                quasis_group = quasis_group + ":";
            } else if !last_iteration {
                quasis_group = quasis_group + ".";
            }
        }
    }

    // When the template literal ends with a quasis
    if !quasis_group.is_empty() {
        quasis.push(TplElement {
            span: span,
            tail: false,
            cooked: Some(quasis_group.clone().into()),
            raw: quasis_group.clone().into(),
        });
    }

    return Expr::Tpl(Tpl {
        exprs: exprs,
        quasis: quasis,
        span: span,
    });
}
