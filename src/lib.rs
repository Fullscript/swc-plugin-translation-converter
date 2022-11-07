use swc_core::ecma::ast::{KeyValueProp, ReturnStmt};
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};
use swc_core::{
    common::Spanned,
    ecma::{
        ast::{CallExpr, ExprOrSpread, Program},
        transforms::testing::test,
        visit::{as_folder, FoldWith, VisitMut, VisitMutWith},
    },
};

#[cfg(test)]
use swc_ecma_parser::{Syntax, TsConfig};

mod utils;
pub use crate::utils::helpers;

pub struct TransformVisitor;

impl VisitMut for TransformVisitor {
    fn visit_mut_call_expr(&mut self, call_expr: &mut CallExpr) {
        // required to ensure that other visit_mut fn are called for children
        call_expr.visit_mut_children_with(self);

        if helpers::is_t_expression(call_expr) {
            // if arguments are empty, skip, nothing to do
            if call_expr.args.is_empty() {
                return;
            }

            // we only ever care about the first argument in a t expression
            // second argument could be variables: t(l.common.foobar, { count });
            // when second argument is a translation, that is handled by visit_mut_key_value_prop
            let arg = call_expr.args.get_mut(0).unwrap();

            if arg.expr.is_member() {
                let member_expr = arg.expr.as_member().unwrap();

                // verify that the member_expression has l before converting
                if helpers::has_child_l(member_expr) {
                    let translation_value = helpers::serialize_l_expr(member_expr, &mut vec![]);

                    let expr = helpers::build_translation_expr(
                        translation_value,
                        call_expr.args[0].span(),
                    );

                    call_expr.args[0] = ExprOrSpread {
                        spread: None,
                        expr: Box::new(expr),
                    }
                }
            } else if arg.expr.is_cond() {
                // for cases where nested ternaries t(something ? l.common.foo1 : l.common.foo2);
                let cond_expr = arg.expr.as_mut_cond().unwrap();

                if cond_expr.cons.is_member() {
                    let member_expr = cond_expr.cons.as_member().unwrap();

                    // verify that the member_expression has l before converting
                    if helpers::has_child_l(member_expr) {
                        let translation_value = helpers::serialize_l_expr(member_expr, &mut vec![]);

                        let expr = helpers::build_translation_expr(
                            translation_value,
                            cond_expr.cons.span(),
                        );

                        cond_expr.cons = Box::new(expr)
                    }
                }

                if cond_expr.alt.is_member() {
                    let member_expr = cond_expr.alt.as_member().unwrap();

                    // verify that the member_expression has l before converting
                    if helpers::has_child_l(member_expr) {
                        let translation_value = helpers::serialize_l_expr(member_expr, &mut vec![]);

                        let expr = helpers::build_translation_expr(
                            translation_value,
                            cond_expr.alt.span(),
                        );

                        cond_expr.alt = Box::new(expr)
                    }
                }
            }
        }
    }

    fn visit_mut_return_stmt(&mut self, return_stmt: &mut ReturnStmt) {
        // required to ensure that other visit_mut fn are called for children
        return_stmt.visit_mut_children_with(self);

        if return_stmt.arg.is_some() {
            // we only ever care about the first argument in a t expression
            // second argument could be variables: t(l.common.foobar, { count });
            let arg = return_stmt.arg.clone().unwrap();

            if arg.is_member() {
                let member_expr = arg.as_member().unwrap();

                // verify that the member_expression has l before converting
                if helpers::has_child_l(member_expr) {
                    let translation_value = helpers::serialize_l_expr(member_expr, &mut vec![]);

                    let expr =
                        helpers::build_translation_expr(translation_value, return_stmt.arg.span());

                    return_stmt.arg = Some(Box::new(expr));
                }
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

            // verify that the member_expression has l before converting
            if helpers::has_child_l(member_expr) {
                let translation_value = helpers::serialize_l_expr(member_expr, &mut vec![]);

                let expr =
                    helpers::build_translation_expr(translation_value, key_value_prop.value.span());

                key_value_prop.value = Box::new(expr);
            }
        }
    }
}

#[plugin_transform]
pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
    program.fold_with(&mut as_folder(TransformVisitor))
}

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    converts_l_member_expressions_inside_of_t_functions,
    r#"t(l.common.fooBar);"#,
    r#"t(common:fooBar);"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    converts_l_member_expressions_inside_of_t_func_with_variables,
    r#"t(l.common.fooBar, { userName });"#,
    r#"t(common:fooBar, { userName });"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    converts_l_to_template_literal_member_expressions,
    r#"const bar = 'cat';t(l.common.foo[bar]);"#,
    r#"const bar = 'cat';t(`common:foo.${bar}`);"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    converts_l_template_literal_member_expressions_with_variable_namespace,
    r#"t(l[common].foo[bar]);"#,
    r#"t(`${common}:foo.${bar}`);"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    converts_l_that_is_part_of_ternary,
    r#"t(something ? l.user.foo : l.user.bar);"#,
    r#"t(something ? user:foo : user:bar);"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    converts_l_that_is_outside_of_t_inside_a_function,
    r#"
    const testFunc = () => {
      return l.userName.foo;
    }
    "#,
    r#"
    const testFunc = () => {
      return userName:foo;
    }
    "#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    converts_l_with_many_nested_namesapces,
    r#"t(l.clerk.one.two.three.four);"#,
    r#"t(clerk:one.two.three.four);"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    converts_nested_l_member_expression,
    r#"t(l.userName.bla, { label: l.userName.label });"#,
    r#"t(userName:bla, { label: userName:label });"#
);

test!(
    Default::default(),
    |_| as_folder(TransformVisitor),
    does_not_convert_member_expressions_that_do_not_start_with_l,
    r#"t(b.userName.bla);"#,
    r#"t(b.userName.bla);"#
);

test!(
    Syntax::Typescript(TsConfig {
        tsx: true,
        ..Default::default()
    }),
    |_| as_folder(TransformVisitor),
    converts_nested_t_functions,
    r#"
    <Component>
      {t(l.common.foo1, {
        label: t(l.common.foo2[bar]),
      })}
    </Component>
    "#,
    r#"
    <Component>
      {t(common:foo1, {
        label: t(`common:foo2.${bar}`),
      })}
    </Component>
    "#
);
