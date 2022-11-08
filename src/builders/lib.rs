use swc_core::{
    common::Span,
    ecma::ast::{Expr, Ident, JSXExpr, Lit, MemberExpr, Str, Tpl, TplElement},
};

use crate::builders::{serializers, utils};

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
    let translation_value = serializers::member_expr(member, &mut vec![]);

    // If builders::serializers::l::expr could not generate a String representation of Member it returns ""
    // This means that the translation l.common is invalid
    if translation_value == "" {
        return None;
    }

    // translation_value contains a ${ we need to generate an Expr::Tpl
    if translation_value.contains("${") {
        let template_expr = expr_tpl(translation_value, span);
        return Some(Box::new(template_expr));
    }

    // translation_value does not contain an interpolated value so we generate a Expr::Lit
    let expr = expr_lit(translation_value, span);

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
fn expr_lit(translation_value: String, span: Span) -> Expr {
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
fn expr_tpl(translation_value: String, span: Span) -> Expr {
    let split_translation = translation_value.split("${").collect::<Vec<_>>();

    // quasis being the portion the appears before the interpolated property
    // ex: for "common:foo.${bar}" quasis would be "common:foo."
    let quasis = split_translation.get(0).unwrap();

    // For the above described example, expr would be "bar" after stripping ${}
    let expr = split_translation.get(1).unwrap().replace("}", "");

    // TemplateLiterals are based on TplElement where each TplElement represents a quasis
    // In this case it would represent "common:foo." in an AST friendly format
    let tpl_element = TplElement {
        span: span,
        tail: false,
        cooked: Some(quasis.clone().into()),
        raw: quasis.clone().into(),
    };

    // All TemplateLiterals seem to require a "tail: true" TplElement to close off the quasis before the expr
    // Otherwise the plugin will panic at the disco
    let tail_element = TplElement {
        span: span,
        tail: true,
        cooked: Some("".into()),
        raw: "".into(),
    };

    let tpl = Tpl {
        exprs: vec![Box::new(Ident::new(expr.into(), span).into())],
        quasis: vec![tpl_element, tail_element],
        span: span,
    };

    return Expr::Tpl(tpl);
}
