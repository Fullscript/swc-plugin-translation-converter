use swc_core::ecma::ast::CallExpr;

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

#[cfg(test)]
mod tests {
    mod is_t_expression {
        use swc_core::{
            common::{BytePos, Span, SyntaxContext},
            ecma::ast::{CallExpr, Callee, Ident},
        };

        use crate::utils::is_t_expression;

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
}
