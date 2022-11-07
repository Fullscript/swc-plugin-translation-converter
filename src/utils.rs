pub mod helpers {
    use swc_core::{
        common::Span,
        ecma::ast::{CallExpr, Expr, Ident, Lit, MemberExpr, Str, Tpl, TplElement},
    };

    pub fn has_child_l(member_expr: &MemberExpr) -> bool {
        if member_expr.obj.is_member() {
            return has_child_l(member_expr.obj.as_member().unwrap());
        } else if member_expr.obj.is_ident() {
            let ident = member_expr.obj.as_ident().unwrap();

            return &ident.sym as &str == "l";
        }

        return false;
    }

    pub fn is_t_expression(call_expr: &CallExpr) -> bool {
        if call_expr.callee.is_expr() {
            let expr = call_expr.callee.as_expr().unwrap();

            if expr.is_ident() {
                let ident = expr.as_ident().unwrap();

                if &ident.sym as &str == "t" {
                    return true;
                }
            }
        }

        return false;
    }

    pub fn serialize_l_expr(member_expr: &MemberExpr, identifiers: &mut Vec<String>) -> String {
        if member_expr.obj.is_ident() {
            let ident = member_expr.obj.as_ident().unwrap();

            // if not l, we add to the list of identifiers
            if &ident.sym as &str != "l" {
                identifiers.push(ident.sym.to_string());
            }
        } else if member_expr.obj.is_member() {
            // recursively call collecting identifiers
            serialize_l_expr(member_expr.obj.as_member().unwrap(), identifiers);
        }

        if member_expr.prop.is_ident() {
            let ident = member_expr.prop.as_ident().unwrap();
            identifiers.push(ident.sym.to_string());
        } else if member_expr.prop.is_computed() {
            // handle else condition for cases where l[common].foobar
            let computed = member_expr.prop.as_computed().unwrap();

            if computed.expr.is_ident() {
                let ident = computed.expr.as_ident().unwrap();
                identifiers.push(format!("${{{}}}", ident.sym.to_string()));
            }
        }

        if identifiers.len() >= 2 {
            let (namespace_group, properties) = identifiers.split_at(2);

            // Case where it's just l.common.foobar
            if properties.is_empty() {
                return namespace_group.join(":");
            } else {
                // case where it's l.common.foo1.foo2...
                return format!("{}.{}", namespace_group.join(":"), properties.join("."));
            }
        } else {
            // TODO: handle this properly in the lib.rs file
            return "".to_string();
        }
    }

    pub fn build_translation_expr(translation_value: String, span: Span) -> Expr {
        if translation_value.contains("${") {
            let split_translation = translation_value.split("${").collect::<Vec<_>>();
            let quasis = split_translation.get(0).unwrap();
            let expr = split_translation.get(1).unwrap().replace("}", "");

            let tpl_element = TplElement {
                span: span,
                tail: false,
                cooked: Some(quasis.clone().into()),
                raw: quasis.clone().into(),
            };

            // Always need a trailing element otherwise it panics at the disco
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

        let translation_raw = format!("{}", translation_value);

        let string_literal = Str {
            raw: Some(translation_raw.clone().into()),
            value: translation_raw.into(),
            span: span,
        };

        return Expr::Lit(Lit::Str(string_literal));
    }
}

#[cfg(test)]
mod tests {
    mod has_child_l {
        use swc_core::{
            common::{BytePos, Span, SyntaxContext},
            ecma::ast::{Ident, MemberExpr, MemberProp},
        };

        use crate::helpers::has_child_l;

        #[test]
        fn returns_true_when_starts_with_l() {
            // l.common
            let span = Span::new(BytePos(0), BytePos(0), SyntaxContext::empty()).into();
            let translation_namespace = Ident::new("common".into(), span).into();

            let translation_member_expr = MemberExpr {
                obj: Box::new(Ident::new("l".into(), span).into()),
                prop: MemberProp::Ident(translation_namespace),
                span,
            };

            assert_eq!(has_child_l(&translation_member_expr), true);
        }

        #[test]
        fn returns_false_when_does_not_contain_l() {
            // data.currentPatient
            let span = Span::new(BytePos(0), BytePos(0), SyntaxContext::empty()).into();
            let current_patient_ident = Ident::new("currentPatient".into(), span).into();

            let data_member_expr = MemberExpr {
                obj: Box::new(Ident::new("data".into(), span).into()),
                prop: MemberProp::Ident(current_patient_ident),
                span,
            };

            assert_eq!(has_child_l(&data_member_expr), false);
        }

        #[test]
        fn returns_false_when_it_does_not_start_with_l() {
            // common.l.foobar
            let span = Span::new(BytePos(0), BytePos(0), SyntaxContext::empty()).into();
            let foobar_ident = Ident::new("foobar".into(), span).into();
            let l_ident = Ident::new("l".into(), span).into();
            let common_ident = Ident::new("common".into(), span).into();

            let nested_member_expr = MemberExpr {
                obj: Box::new(common_ident),
                prop: MemberProp::Ident(l_ident),
                span,
            };

            let member_expr = MemberExpr {
                obj: Box::new(nested_member_expr.into()),
                prop: MemberProp::Ident(foobar_ident),
                span,
            };

            assert_eq!(has_child_l(&member_expr), false);
        }
    }

    mod is_t_expression {
        use swc_core::{
            common::{BytePos, Span, SyntaxContext},
            ecma::ast::{CallExpr, Callee, Ident},
        };

        use crate::helpers::is_t_expression;

        #[test]
        fn returns_true_when_expression_name_is_t() {
            // t()
            let span = Span::new(BytePos(0), BytePos(0), SyntaxContext::empty()).into();
            let t_ident = Ident::new("t".into(), span).into();

            let t_expr = CallExpr {
                args: vec![],
                callee: Callee::Expr(t_ident),
                span,
                type_args: Option::None,
            };

            assert_eq!(is_t_expression(&t_expr), true);
        }

        #[test]
        fn returns_false_when_expression_name_is_not_t() {
            // notT()
            let span = Span::new(BytePos(0), BytePos(0), SyntaxContext::empty()).into();
            let not_t_ident = Ident::new("notT".into(), span).into();

            let not_t_expr = CallExpr {
                args: vec![],
                callee: Callee::Expr(not_t_ident),
                span,
                type_args: Option::None,
            };

            assert_eq!(is_t_expression(&not_t_expr), false);
        }
    }

    mod serialize_l_expr {
        use swc_core::{
            common::{BytePos, Span, SyntaxContext},
            ecma::ast::{Ident, MemberExpr, MemberProp},
        };

        use crate::helpers::serialize_l_expr;

        #[test]
        fn returns_serialized_translation_conversion() {
            // l.common.foobar
            let span = Span::new(BytePos(0), BytePos(0), SyntaxContext::empty()).into();
            let l_ident = Ident::new("l".into(), span).into();
            let common_ident = Ident::new("common".into(), span).into();
            let foobar_ident = Ident::new("foobar".into(), span).into();

            let nested_member_expr = MemberExpr {
                obj: Box::new(l_ident),
                prop: MemberProp::Ident(common_ident),
                span,
            };

            let member_expr = MemberExpr {
                obj: Box::new(nested_member_expr.into()),
                prop: MemberProp::Ident(foobar_ident),
                span,
            };

            assert_eq!(serialize_l_expr(&member_expr, &mut vec![]), "common:foobar");
        }

        #[test]
        fn returns_serialized_translation_conversion_for_long_expressions() {
            // l.common.foo1.foo2.foo3
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

            let member_expr = MemberExpr {
                obj: Box::new(nested_member_expr1.into()),
                prop: MemberProp::Ident(foo3_ident),
                span,
            };

            assert_eq!(
                serialize_l_expr(&member_expr, &mut vec![]),
                "common:foo1.foo2.foo3"
            );
        }

        #[test]
        fn returns_empty_string_if_invalid_translation() {
            // l.common
            let span = Span::new(BytePos(0), BytePos(0), SyntaxContext::empty()).into();
            let l_ident = Ident::new("l".into(), span).into();
            let common_ident = Ident::new("common".into(), span).into();

            let member_expr = MemberExpr {
                obj: Box::new(l_ident),
                prop: MemberProp::Ident(common_ident),
                span,
            };

            assert_eq!(serialize_l_expr(&member_expr, &mut vec![]), "");
        }

        // Write test for computed case
    }
}
