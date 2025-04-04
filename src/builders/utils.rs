use swc_ecma_ast::MemberExpr;

pub fn has_child_l(member_expr: &MemberExpr) -> bool {
    if member_expr.obj.is_member() {
        return has_child_l(member_expr.obj.as_member().unwrap());
    } else if member_expr.obj.is_ident() {
        let ident = member_expr.obj.as_ident().unwrap();

        return &ident.sym as &str == "l";
    }

    return false;
}

#[cfg(test)]
mod tests {
    mod has_child_l {
        use swc_core::common::{BytePos, Span, SyntaxContext};
        use swc_ecma_ast::{Ident, MemberExpr, MemberProp};

        use crate::builders::utils::has_child_l;

        #[test]
        fn returns_true_when_starts_with_l() {
            // l.common
            let span = Span::new(BytePos(0), BytePos(0)).into();
            let translation_namespace =
                Ident::new("common".into(), span, SyntaxContext::empty()).into();

            let translation_member_expr = MemberExpr {
                obj: Box::new(Ident::new("l".into(), span, SyntaxContext::empty()).into()),
                prop: MemberProp::Ident(translation_namespace),
                span,
            };

            assert_eq!(has_child_l(&translation_member_expr), true);
        }

        #[test]
        fn returns_false_when_does_not_contain_l() {
            // data.currentPatient
            let span = Span::new(BytePos(0), BytePos(0)).into();
            let current_patient_ident =
                Ident::new("currentPatient".into(), span, SyntaxContext::empty()).into();

            let data_member_expr = MemberExpr {
                obj: Box::new(Ident::new("data".into(), span, SyntaxContext::empty()).into()),
                prop: MemberProp::Ident(current_patient_ident),
                span,
            };

            assert_eq!(has_child_l(&data_member_expr), false);
        }

        #[test]
        fn returns_false_when_it_does_not_start_with_l() {
            // common.l.foobar
            let span = Span::new(BytePos(0), BytePos(0)).into();
            let foobar_ident = Ident::new("foobar".into(), span, SyntaxContext::empty()).into();
            let l_ident = Ident::new("l".into(), span, SyntaxContext::empty()).into();
            let common_ident = Ident::new("common".into(), span, SyntaxContext::empty()).into();

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
}
