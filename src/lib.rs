use builders::lib::jsx_expr;
use swc_core::{
    common::Spanned,
    ecma::transforms::testing::test,
    plugin::{plugin_transform, proxies::TransformPluginProgramMetadata},
};
use swc_ecma_ast::{
    CallExpr, CondExpr, ExprOrSpread, JSXExpr, JSXExprContainer, KeyValueProp, Program, ReturnStmt,
};
use swc_ecma_visit::{visit_mut_pass, VisitMut, VisitMutWith};

mod builders {
    pub mod lib;
    pub mod serializers;
    pub mod utils;
}

struct TranslationConverterVisitor;

impl VisitMut for TranslationConverterVisitor {
    // t(l.common.foo...);
    fn visit_mut_call_expr(&mut self, call_expr: &mut CallExpr) {
        // required to ensure that other visit_mut fn are called for children
        call_expr.visit_mut_children_with(self);

        // if arguments are empty, skip, nothing to do
        // This would only happen if you used t like so t();
        // Not sure why you'd do that but hey
        if call_expr.args.is_empty() {
            return;
        }

        // We must loop through all args of the call_expr for cases like so:
        // ex: mobileHeaderContent(faChevronDown, l.common.ShowOrderSummary)
        for (i, arg) in call_expr.args.clone().iter().enumerate() {
            // t(l.common.foobar);
            if arg.expr.is_member() {
                let member_expr = arg.expr.as_member().unwrap();
                let box_expr = builders::lib::box_expr(member_expr, arg.span());

                if box_expr.is_some() {
                    call_expr.args[i] = ExprOrSpread {
                        spread: None,
                        expr: box_expr.unwrap(),
                    }
                }
            }
        }
    }

    // for cases where the translation is nested inside a conditional statement somewhere
    // isFoo ? l.common.foo : l.common.bar;
    fn visit_mut_cond_expr(&mut self, cond_expr: &mut CondExpr) {
        // required to ensure that other visit_mut fn are called for children
        cond_expr.visit_mut_children_with(self);

        // cons here being the first result in our ternary if truthy
        // from the above example comment that would be l.common.foo
        if cond_expr.cons.is_member() {
            let member_expr = cond_expr.cons.as_member().unwrap();
            let box_expr = builders::lib::box_expr(member_expr, cond_expr.cons.span());

            if box_expr.is_some() {
                cond_expr.cons = box_expr.unwrap();
            }
        }

        // alt here being the second result in our ternary if falsy
        // from the above example comment that would be l.common.bar
        if cond_expr.alt.is_member() {
            let member_expr = cond_expr.alt.as_member().unwrap();
            let box_expr = builders::lib::box_expr(member_expr, cond_expr.alt.span());

            if box_expr.is_some() {
                cond_expr.alt = box_expr.unwrap();
            }
        }
    }

    // for cases where the translation is returned as part of a function and not nested inside of a t call
    // const func = () => l.common.foobar;
    fn visit_mut_return_stmt(&mut self, return_stmt: &mut ReturnStmt) {
        // required to ensure that other visit_mut fn are called for children
        return_stmt.visit_mut_children_with(self);

        // If there's no arguments to our return statement, we don't need to do anything
        if return_stmt.arg.is_none() {
            return;
        }

        // we only care about the first argument in a t expression
        // second argument could be variables: t(l.common.foobar, { count });
        // when second argument is a translation, that is handled by visit_mut_key_value_prop
        let arg = return_stmt.arg.clone().unwrap();

        if arg.is_member() {
            let member_expr = arg.as_member().unwrap();
            let box_expr = builders::lib::box_expr(member_expr, return_stmt.arg.span());

            if box_expr.is_some() {
                return_stmt.arg = box_expr;
            }
        }
    }

    // for cases where translation is used as a variable
    // t(l.common.foobar, { label: l.common.label });
    fn visit_mut_key_value_prop(&mut self, key_value_prop: &mut KeyValueProp) {
        // required to ensure that other visit_mut fn are called for children
        key_value_prop.visit_mut_children_with(self);

        if key_value_prop.value.is_member() {
            let member_expr = key_value_prop.value.as_member().unwrap();
            let box_expr = builders::lib::box_expr(member_expr, key_value_prop.value.span());

            if box_expr.is_some() {
                key_value_prop.value = box_expr.unwrap();
            }
        }
    }

    fn visit_mut_jsx_expr_container(&mut self, jsx_expr_cont: &mut JSXExprContainer) {
        // required to ensure that other visit_mut fn are called for children
        jsx_expr_cont.visit_mut_children_with(self);

        match jsx_expr_cont.expr.clone() {
            JSXExpr::JSXEmptyExpr(_) => return,
            JSXExpr::Expr(expr) => {
                if expr.is_member() {
                    let member_expr = expr.as_member().unwrap();
                    let box_expr = builders::lib::box_expr(member_expr, expr.span());

                    if box_expr.is_some() {
                        jsx_expr_cont.expr = jsx_expr(member_expr, jsx_expr_cont.span()).unwrap();
                    }
                }
            }
        }
    }
}

#[plugin_transform]
pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
    let mut program = program;
    visit_mut_pass(TranslationConverterVisitor).visit_mut_program(&mut program);
    program
}

#[cfg(test)]
use swc_ecma_parser::Syntax;

#[cfg(test)]
fn config() -> Syntax {
    return swc_ecma_parser::Syntax::Typescript(swc_ecma_parser::TsSyntax {
        tsx: true,
        decorators: false,
        dts: true,
        no_early_errors: true,
        disallow_ambiguous_jsx_like: true,
    });
}

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_l_member_expressions_inside_of_t_functions,
    r#"t(l.common.fooBar);"#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_l_member_expressions_inside_of_t_func_with_variables,
    r#"t(l.common.fooBar, { userName });"#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_l_to_template_literal_member_expressions,
    r#"
    const bar = 'cat';
    t(l.common.foo[bar]);
    "#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_l_template_literal_member_expressions_with_variable_namespace,
    r#"t(l[common].foo[bar]);"#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_l_template_literal_member_expressions_with_trailing_quasis,
    r#"t(l[common].foo1[bar1].foo2[bar2].foo3);"#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_l_template_literal_member_expressions_with_expression_in_middle,
    r#"t(l.common.foo1[bar1].foo3);"#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_l_that_is_part_of_ternary,
    r#"t(something ? l.user.foo : l.user.bar);"#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_l_that_is_outside_of_t_inside_a_function,
    r#"
    const testFunc = () => {
      return l.userName.foo;
    }
    "#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_cond_expr_with_l_in_functions,
    r#"
    const testFunc = () => {
      return true ? l.userName.foo : l.userName.bar;
    }
    "#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_l_with_many_nested_namesapces,
    r#"t(l.clerk.one.two.three.four);"#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_nested_l_member_expression,
    r#"t(l.userName.bla, { label: l.userName.label });"#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    does_not_convert_member_expressions_that_do_not_start_with_l,
    r#"t(b.userName.bla);"#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_nested_t_functions,
    r#"<Component>{t(l.common.foo1, { label: t(l.common.foo2[bar]) })}</Component>"#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_trans_i18n_key,
    r#"<Trans i18nKey={l.common.foobar}>hello world</Trans>"#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_trans_nested_in_call_expr_in_jsx_expr,
    r#"
    <Collapsible
      trigger={mobileHeaderContent(faChevronDown, l.common.ShowOrderSummary)}
      triggerWhenOpen={mobileHeaderContent(faChevronUp, l.common.HideOrderSummary)}
    />
    "#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_l_nested_in_template_literals,
    r#"
    const variable = `${obj.property} ${t(l.common.foo1, {
      count: obj.property,
    })} ($${price.toFixed(2)} ${t(l.common.foo2)})`;
    "#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_l_template_literal_member_expressions_with_nested_conditional,
    r#"t(l.common.foo[bar ? "bar" : "baz"]);"#
);

test!(
    config(),
    |_| visit_mut_pass(TranslationConverterVisitor),
    converts_l_template_literal_member_expressions_with_computed_member_expressions,
    r#"
    const bar = {namespace: 'common', cat: 'kitty'};
    t(l[bar.namespace].foo[bar.cat]);
    "#
);
