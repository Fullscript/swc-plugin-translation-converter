use swc_core::ecma::ast::MemberExpr;

/// Serializes a MemberExpression that contains an l translation object
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
/// /// assert_eq!(`common:foo.${bar}`, translation_value);
/// ```
pub fn member_expr(member: &MemberExpr, identifiers: &mut Vec<String>) -> String {
    // Case where member_expr is a nested collection of MemberExpr
    // We need to recursively continue down the AST collecting all Ident as we go
    // ex: l.common.foobar
    if member.obj.is_member() {
        member_expr(member.obj.as_member().unwrap(), identifiers);
    }

    // If prop is an Ident, add it to the list of identifiers to convert into a StringLiteral
    if member.prop.is_ident() {
        let ident = member.prop.as_ident().unwrap();
        identifiers.push(ident.sym.to_string());
    // If prop is a computed expr we need to convert [bar] into ${bar} before adding to the list of identifiers
    // ex: l.common.foo[bar]
    } else if member.prop.is_computed() {
        let computed = member.prop.as_computed().unwrap();

        if computed.expr.is_ident() {
            let ident = computed.expr.as_ident().unwrap();
            identifiers.push(format!("${{{}}}", ident.sym.to_string()));
        }
    } // handle else condition for cases where l[common].foobar

    return concatenate_identifiers(identifiers.clone());
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
fn concatenate_identifiers(identifiers: Vec<String>) -> String {
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
            return namespace_group.join(":");
        } else {
            // Case where identifiers was ["common", "foo1", "foo2", ...] originally
            // - namespace_group = ["common", "foo1"]
            // - properties = ["foo2", ...]
            return format!("{}.{}", namespace_group.join(":"), properties.join("."));
        }
    } else {
        return "".to_string();
    }
}

#[cfg(test)]
mod tests {
    use swc_core::{
        common::{BytePos, Span, SyntaxContext},
        ecma::ast::{Ident, MemberExpr, MemberProp},
    };

    use crate::builders::serializers::member_expr;

    #[test]
    // l.common.foobar
    fn returns_serialized_translation_conversion() {
        let span = Span::new(BytePos(0), BytePos(0), SyntaxContext::empty()).into();
        let l_ident = Ident::new("l".into(), span).into();
        let common_ident = Ident::new("common".into(), span).into();
        let foobar_ident = Ident::new("foobar".into(), span).into();

        let nested_member_expr = MemberExpr {
            obj: Box::new(l_ident),
            prop: MemberProp::Ident(common_ident),
            span,
        };

        let member = MemberExpr {
            obj: Box::new(nested_member_expr.into()),
            prop: MemberProp::Ident(foobar_ident),
            span,
        };

        assert_eq!(member_expr(&member, &mut vec![]), "common:foobar");
    }

    #[test]
    // l.common.foo1.foo2.foo3
    fn returns_serialized_translation_conversion_for_long_expressions() {
        let span = Span::new(BytePos(0), BytePos(0), SyntaxContext::empty()).into();
        let l_ident = Ident::new("l".into(), span).into();
        let common_ident = Ident::new("common".into(), span).into();
        let foo1_ident = Ident::new("foo1".into(), span).into();
        let foo2_ident = Ident::new("foo2".into(), span).into();
        let foo3_ident = Ident::new("foo3".into(), span).into();

        let nested_member_expr3 = MemberExpr {
            obj: l_ident,
            prop: common_ident,
            span,
        };

        let nested_member_expr2 = MemberExpr {
            obj: Box::new(nested_member_expr3.into()),
            prop: MemberProp::Ident(foo1_ident),
            span,
        };

        let nested_member_expr1 = MemberExpr {
            obj: Box::new(nested_member_expr2.into()),
            prop: MemberProp::Ident(foo2_ident),
            span,
        };

        let member = MemberExpr {
            obj: Box::new(nested_member_expr1.into()),
            prop: MemberProp::Ident(foo3_ident),
            span,
        };

        assert_eq!(member_expr(&member, &mut vec![]), "common:foo1.foo2.foo3");
    }

    #[test]
    // l.common
    fn returns_empty_string_if_invalid_translation() {
        let span = Span::new(BytePos(0), BytePos(0), SyntaxContext::empty()).into();
        let l_ident = Ident::new("l".into(), span).into();
        let common_ident = Ident::new("common".into(), span).into();

        let member = MemberExpr {
            obj: Box::new(l_ident),
            prop: MemberProp::Ident(common_ident),
            span,
        };

        assert_eq!(member_expr(&member, &mut vec![]), "");
    }

    // TODO: Ryan - Write tests for computed case
}
