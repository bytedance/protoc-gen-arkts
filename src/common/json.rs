 /**
  * Copyright 2024 ByteDance and/or its affiliates
  *
  * Original Files：protoc-gen-ts (https://github.com/thesayyn/protoc-gen-ts)
  * Copyright (c) 2024 Sahin Yort
  * SPDX-License-Identifier: MIT 
 */

use std::fmt::{format, Display, LowerExp};
use std::vec;

use crate::context::Syntax;
use crate::descriptor::field_descriptor_proto::Type;
use crate::descriptor::{DescriptorProto, FileDescriptorProto};
use crate::{context::Context, descriptor::FieldDescriptorProto};

use super::field::FieldAccessorFn;
use convert_case::{Case, Casing};
use protobuf::well_known_types::struct_::value;
use swc_common::DUMMY_SP;
use swc_ecma_ast::{
    ArrayLit, ArrayPat, BinaryOp, BlockStmt, ClassMember, ClassMethod, Expr, Function, MethodKind, ObjectLit, Param, Pat, PatOrExpr, PropName, Stmt, TsType, UnaryOp, TsNonNullExpr
};
use swc_ecma_utils::{quote_ident, quote_str};

pub(crate) fn json_key_name_field_member(field: &FieldDescriptorProto) -> Expr {
    crate::member_expr!("json", field.json_key_name())
}

pub(crate) fn name_field_member(field: &FieldDescriptorProto) -> Expr {
    crate::member_expr!("json", field.name())
}

impl FieldDescriptorProto {
    fn json_repr_for_well_known_type(&self) -> &str {
        match self.type_name().trim_start_matches(".") {
            "google.protobuf.BoolValue" => "boolean",
            "google.protobuf.BytesValue" => "string",
            "google.protobuf.DoubleValue" => "number",
            "google.protobuf.Duration" => "string",
            "google.protobuf.FieldMask" => "string",
            "google.protobuf.FloatValue" => "number",
            "google.protobuf.Int32Value" => "number",
            "google.protobuf.Int64Value" => "number|string",
            "google.protobuf.ListValue" => "array",
            "google.protobuf.StringValue" => "string",
            "google.protobuf.Timestamp" => "string",
            "google.protobuf.UInt32Value" => "number",
            "google.protobuf.UInt64Value" => "number|string",
            "google.protobuf.Value" => "unknown",
            "google.protobuf.NullValue" => "null",
            _ => "object",
        }
    }
    fn typeof_expr_for_well_known_type(&self, accessor: FieldAccessorFn) -> Expr {
        self.typeof_expr_for_type(accessor, self.json_repr_for_well_known_type())
    }

    fn typeof_expr_for_type(&self, accessor: FieldAccessorFn, typ: &str) -> Expr {
        match typ {
            "unknown" => crate::paren_expr!(crate::chain_bin_exprs_or!(
                crate::typeof_unary_expr!(accessor(self).into(), "number"),
                crate::typeof_unary_expr!(accessor(self).into(), "string"),
                crate::typeof_unary_expr!(accessor(self).into(), "boolean"),
                crate::typeof_unary_expr!(accessor(self).into(), "object"),
                crate::bin_expr!(
                    accessor(self).into(),
                    quote_ident!("null").into(),
                    BinaryOp::EqEqEq
                )
            )),
            "number|string" => crate::paren_expr!(crate::chain_bin_exprs_or!(
                crate::typeof_unary_expr!(accessor(self).into(), "number"),
                crate::typeof_unary_expr!(accessor(self).into(), "string")
            )),
            "array" => crate::call_expr!(
                crate::member_expr!("Array", "isArray"),
                vec![crate::expr_or_spread!(accessor(self).into())]
            ),
            "null" => crate::bin_expr!(
                accessor(self).into(),
                quote_ident!("null").into(),
                BinaryOp::EqEqEq
            ),
            typ => crate::typeof_unary_expr!(accessor(self).into(), typ),
        }
    }

    pub(self) fn infinity_and_nan_check(&self, accessor: FieldAccessorFn) -> Expr {
        crate::chain_bin_exprs_or!(
            crate::bin_expr!(
                accessor(self),
                crate::lit_str!("NaN").into(),
                BinaryOp::EqEqEq
            ),
            crate::bin_expr!(
                accessor(self),
                crate::lit_str!("Infinity").into(),
                BinaryOp::EqEqEq
            ),
            crate::bin_expr!(
                accessor(self),
                crate::lit_str!("-Infinity").into(),
                BinaryOp::EqEqEq
            )
        )
    }

    pub(self) fn min_max_check<T>(&self, accessor: FieldAccessorFn, min: T, max: T) -> Expr
    where
        T: Display + LowerExp,
    {
        crate::paren_expr!(crate::chain_bin_exprs_and!(
            crate::bin_expr!(
                accessor(self),
                quote_ident!(format!("{:+e}", min)).into(),
                BinaryOp::GtEq
            ),
            crate::bin_expr!(
                accessor(self),
                quote_ident!(format!("{:+e}", max)).into(),
                BinaryOp::LtEq
            )
        ))
    }

    pub(self) fn min_max_check_bigint<T>(&self, accessor: FieldAccessorFn, min: T, max: T) -> Expr
    where
        T: Into<num_bigint::BigInt>,
    {
        crate::paren_expr!(crate::chain_bin_exprs_and!(
            crate::bin_expr!(
                accessor(self),
                crate::lit_bigint!(min.into()).into(),
                BinaryOp::GtEq
            ),
            crate::bin_expr!(
                accessor(self),
                crate::lit_bigint!(max.into()).into(),
                BinaryOp::LtEq
            )
        ))
    }

    pub(self) fn default_value_bin_expr_for_json(
        &self,
        ctx: &mut Context,
        accessor: FieldAccessorFn,
    ) -> Expr {
        let neq_undefined_check = crate::bin_expr!(
            accessor(self),
            quote_ident!("undefined").into(),
            BinaryOp::NotEqEq
        );

        let neq_null_check = crate::bin_expr!(
            accessor(self),
            quote_ident!("null").into(),
            BinaryOp::NotEqEq
        );

        let neq_null_or_undefined_check = if self.json_repr_for_well_known_type() == "unknown" {
            neq_undefined_check
        } else {
            crate::chain_bin_exprs_and!(neq_null_check, neq_undefined_check)
        };

        let presence_check = if self.has_oneof_index() {
            // for oneof field we have to serialize the value unconditionally even if the value is the default.
            neq_null_or_undefined_check
        } else if self.is_map(ctx) {
            crate::bin_expr!(
                neq_null_or_undefined_check,
                crate::bin_expr!(
                    crate::member_expr_bare!(accessor(self), "size"),
                    Expr::Lit(crate::lit_num!(0)),
                    BinaryOp::NotEqEq
                )
            )
        } else if (self.is_bytes() && ctx.syntax == &Syntax::Proto3) || self.is_repeated() {
            crate::bin_expr!(
                neq_null_or_undefined_check,
                crate::bin_expr!(
                    crate::member_expr_bare!(accessor(self), "length"),
                    Expr::Lit(crate::lit_num!(0)),
                    BinaryOp::NotEqEq
                )
            )
        } else {
            neq_null_or_undefined_check
        };

        let default_expr = self.proto3_default(ctx);

        if default_expr.is_some() && ctx.syntax == &Syntax::Proto3 && !self.has_oneof_index() {
            crate::bin_expr!(
                presence_check,
                crate::bin_expr!(accessor(self), default_expr.unwrap(), BinaryOp::NotEqEq)
            )
        } else {
            presence_check
        }
    }

    pub(self) fn value_check_stmt(&self, ctx: &Context, accessor: FieldAccessorFn) -> Stmt {
        let min_max_check: Option<Expr> = match self.type_() {
            Type::TYPE_FLOAT => Some(self.min_max_check(accessor, f32::MIN, f32::MAX)),
            Type::TYPE_DOUBLE => Some(self.min_max_check(accessor, f64::MIN, f64::MAX)),
            Type::TYPE_UINT32 | Type::TYPE_FIXED32 => {
                Some(self.min_max_check(accessor, u32::MIN, u32::MAX))
            }
            Type::TYPE_UINT64 | Type::TYPE_FIXED64 => {
                Some(self.min_max_check_bigint(accessor, u64::MIN, u64::MAX))
            }
            Type::TYPE_INT32 | Type::TYPE_SFIXED32 | Type::TYPE_SINT32 => {
                Some(self.min_max_check(accessor, i32::MIN, i32::MAX))
            }
            Type::TYPE_INT64 | Type::TYPE_SFIXED64 | Type::TYPE_SINT64 => {
                Some(self.min_max_check_bigint(accessor, i64::MIN, i64::MAX))
            }
            _ => None,
        };

        let num_check = if min_max_check.is_some() {
            Some(crate::chain_bin_exprs_or!(
                self.infinity_and_nan_check(accessor),
                min_max_check.unwrap()
            ))
        } else {
            None
        };

        let typeof_check = if self.is_well_known_message() {
            self.typeof_expr_for_well_known_type(accessor)
        } else if self.is_string() || self.is_bytes() {
            self.typeof_expr_for_type(accessor, "string")
        } else if self.is_booelan() {
            self.typeof_expr_for_type(accessor, "boolean")
        } else if self.is_message() {
            /* also map */
            self.typeof_expr_for_type(accessor, "object")
        } else if self.is_integer() {
            // integer (non-bigint-float-double) needs special check
            crate::chain_bin_exprs_and!(
                crate::paren_expr!(crate::chain_bin_exprs_or!(
                    self.typeof_expr_for_type(accessor, "number"),
                    crate::chain_bin_exprs_and!(
                        self.typeof_expr_for_type(accessor, "string"),
                        crate::bin_expr!(
                            crate::call_expr!(
                                crate::member_expr_bare!(accessor(self), "indexOf"),
                                vec![crate::expr_or_spread!(crate::lit_str!(" ").into())]
                            ),
                            crate::lit_num!(-1).into(),
                            BinaryOp::EqEqEq
                        )
                    )
                )),
                crate::call_expr!(
                    crate::member_expr!("Number", "isInteger"),
                    vec![crate::expr_or_spread!(crate::unary_expr!(
                        accessor(self).into(),
                        UnaryOp::Plus
                    ))]
                )
            )
        } else if self.is_number() {
            self.typeof_expr_for_type(accessor, "number|string")
        } else if self.is_enum() {
            crate::chain_bin_exprs_or!(
                self.typeof_expr_for_type(accessor, "number"),
                crate::chain_bin_exprs_and!(
                    self.typeof_expr_for_type(accessor, "string"),
                    crate::bin_expr!(
                        accessor(self).into(),
                        ctx.lazy_type_ref(self.type_name()).into(),
                        BinaryOp::In
                    )
                )
            )
        } else {
            self.typeof_expr_for_type(accessor, "never!")
        };

        let check = if num_check.is_some() {
            crate::chain_bin_exprs_and!(typeof_check, crate::paren_expr!(num_check.unwrap()))
        } else {
            typeof_check
        };

        crate::if_stmt!(
            crate::unary_expr!(crate::paren_expr!(check)),
            crate::throw_stmt!(crate::new_expr!(
                quote_ident!("Error").into(),
                vec![crate::expr_or_spread!(crate::lit_str!(format!(
                    "illegal value for {}",
                    self.json_key_name()
                ))
                .into())]
            ))
        )
    }

    pub(self) fn json_key_name(&self) -> String {
        if self.has_json_name() {
            self.json_name().to_string()
        } else {
            self.name().to_string()
        }
    }

    pub(self) fn into_to_stringified_map_expr(&self, ctx: &mut Context) -> Expr {
        if self.is_string() {
            return Expr::Ident(quote_ident!(self.name()));
        }
        if self.name() == "key" {
            super::field::to_string_normalizer(&quote_ident!(self.name()).into())
        } else {
            self.into_to_json_expr(ctx, super::field::bare_field_member)
        }
    }

    pub(self) fn into_to_json_expr(
        &self,
        ctx: &mut Context,
        accessor_fn: super::field::FieldAccessorFn,
    ) -> Expr {
        let accessor = accessor_fn(self);
        if self.is_enum() {
            crate::call_expr!(crate::member_expr_bare!(accessor_fn(self), "valueOf"))

            // crate::bin_expr!(
            //     crate::member_expr_computed!(ctx.lazy_type_ref(self.type_name()).into(), accessor),
            //     accessor_fn(self).into(),
            //     BinaryOp::NullishCoalescing
            // )
        } else if self.is_bytes() {
            let base64 = ctx.get_import(ctx.options.base64_package.as_str());
            crate::call_expr!(
                crate::member_expr!(base64, "fromUint8Array"),
                vec![crate::expr_or_spread!(accessor)]
            )
            // crate::call_expr!(
            //     crate::member_expr!(base64, "encode"),
            //     vec![crate::expr_or_spread!(accessor)]
            // )
        } else if self.is_bigint() {
            crate::call_expr!(crate::member_expr_bare!(accessor, "toString"))
        } else if self.is_number() {
            crate::cond_expr!(
                crate::call_expr!(
                    crate::member_expr!("Number", "isFinite"),
                    vec![crate::expr_or_spread!(accessor.clone())]
                ),
                accessor.clone().into(),
                super::field::to_string_normalizer(&accessor)
            )
        } else if self.is_message() && !self.is_map(ctx) {
            crate::call_expr!(crate::member_expr_bare!(accessor, "toJson"))
        } else {
            accessor
        }
    }

    pub(self) fn into_from_json_expr_for_map_key(
        &self,
        ctx: &mut Context,
        accessor_fn: super::field::FieldAccessorFn,
    ) -> Expr {
        let accessor = accessor_fn(self);
        if self.is_booelan() {
            crate::bin_expr!(accessor, quote_str!("true").into(), BinaryOp::EqEqEq)
        } else {
            self.into_from_json_expr(ctx, accessor_fn)
        }
    }

    pub(self) fn into_from_json_expr(
        &self,
        ctx: &mut Context,
        accessor_fn: super::field::FieldAccessorFn,
    ) -> Expr {
        let accessor = accessor_fn(self);
        if self.is_enum() {
            crate::cond_expr!(
                crate::typeof_unary_expr!(accessor_fn(self).into(), "number"),
                accessor_fn(self).into(),
                crate::member_expr_computed!(ctx.lazy_type_ref(self.type_name()).into(), accessor)
            )
        } else if self.is_bytes() {
            let base64 = ctx.get_import(ctx.options.base64_package.as_str());
            crate::call_expr!(
                crate::member_expr!(base64, "toUint8Array"),
                vec![crate::expr_or_spread!(accessor)]
            )
            // crate::call_expr!(
            //     crate::member_expr!(base64, "decode"),
            //     vec![crate::expr_or_spread!(accessor)]
            // )
        } else if self.is_bigint() {
            crate::call_expr!(
                quote_ident!("BigInt").into(),
                vec![crate::expr_or_spread!(accessor)]
            )
        } else if self.is_bigint() {
            crate::call_expr!(
                quote_ident!("BigInt").into(),
                vec![crate::expr_or_spread!(accessor)]
            )
        } else if self.is_number() {
            crate::call_expr!(
                quote_ident!("Number").into(),
                vec![crate::expr_or_spread!(accessor)]
            )
        } else if self.is_message() && !self.is_map(ctx) {
            crate::call_expr!(
                crate::member_expr_bare!(ctx.lazy_type_ref(self.type_name()).into(), "fromJson"),
                vec![crate::expr_or_spread!(accessor)]
            )
        } else {
            accessor
        }
    }
}

impl DescriptorProto {
    fn get_map_field_descriptor_str(&self, field: &FieldDescriptorProto) -> &str {
        if field.is_string() {
            "string"
        } else if field.is_bigint() {
            "bigint"
        } else if field.is_number() {
            "number"
        } else if field.is_booelan() {
            "boolean"
        } else if field.is_bytes() {
            "Uint8Array"
        } else {
            "object"
        }
    }

    fn get_field_descriptor_str(&self, ctx: &mut Context, field: &FieldDescriptorProto) -> String {
            let base = if field.is_string() {
                ": string".to_string()
            } else if field.is_bigint() {
                ": bigint".to_string()
            } else if field.is_number() || field.is_enum() {
                ": number".to_string()
            } else if field.is_booelan() {
                ": boolean".to_string()
            } else if field.is_bytes() {
                ": string".to_string()
            } else if field.is_map(ctx) {
                let descriptor = ctx
                    .get_map_type(field.type_name())
                    .expect(format!("can not find the map type {}", field.type_name()).as_str());
                let key_type = self.get_map_field_descriptor_str(&descriptor.field[0]);
                let value_type = self.get_map_field_descriptor_str(&descriptor.field[1]);
                format!(": Map<{},{}>", key_type, value_type)
            } else {
                ": object".to_string()
            };
            
            if field.is_repeated() && !field.is_map(ctx)  {
                format!("{}[]", base)
            } else {
                base
            }
    }
     

    fn print_to_json_inner(&self, ctx: &mut Context, index: i32, fields: &Vec<&FieldDescriptorProto>) -> ClassMember {
        let mut statements = vec![];

        for field in fields {
            let accessor_fn = if field.is_repeated() && !field.is_map(ctx) {
                super::field::static_field_member
            } else {
                super::field::this_field_member
            };

            let mut value_expr = field.into_to_json_expr(ctx, accessor_fn);
            let mut stmts = vec![];

            if field.is_map(ctx) {
                let descriptor = ctx
                    .get_map_type(field.type_name())
                    .expect(format!("can not find the map type {}", field.type_name()).as_str());
                let key_type: &str = self.get_map_field_descriptor_str(&descriptor.field[0]);
                let valur_type: &str = self.get_map_field_descriptor_str(&descriptor.field[1]);
              
                stmts.push(crate::expr_stmt!(
                    Expr::Ident(quote_ident!(
                        format!("json[\"{}\"] = new Map<{},{}>()", field.name(), key_type, valur_type)))));
                stmts.push(crate::expr_stmt!(crate::call_expr!(
                    crate::member_expr_bare!( crate::member_expr!("this", field.name()), "forEach"),
                    vec![crate::expr_or_spread!(crate::arrow_func!(
                        vec![crate::pat_ident!(quote_ident!("value")), crate::pat_ident!(quote_ident!("key"))],
                        vec![
                            crate::expr_stmt!(crate::call_expr!(
                                crate::member_expr_bare!(
                                    crate::member_expr_computed!(Expr::Ident(quote_ident!("json")), Expr::Ident(quote_ident!(format!("\"{}\"", field.name())))), "set"),
                                vec![
                                    crate::expr_or_spread!(Expr::TsNonNull(TsNonNullExpr {
                                        expr: Box::new(Expr::Ident(quote_ident!("key"))),
                                        span: DUMMY_SP
                                    })),
                                    crate::expr_or_spread!(Expr::TsNonNull(TsNonNullExpr {
                                        expr: Box::new(Expr::Ident(quote_ident!("value"))),
                                        span: DUMMY_SP
                                    })),
                                ]
                            ))
                        ]
                    ))]
                )))
            } else if field.is_repeated() {
                value_expr = crate::call_expr!(
                    crate::member_expr_bare!(crate::member_expr!("this", field.name()), "map"),
                    vec![crate::expr_or_spread!(crate::arrow_func_short!(
                        value_expr,
                        vec![crate::pat_ident!(quote_ident!("r"))]
                    ))]
                );
            }

            if field.is_map(ctx) {
                statements.push(crate::if_stmt!(
                    field.default_value_bin_expr(ctx, super::field::this_field_member),
                    crate::block_stmt!(stmts)
                ))
            } else {
                statements.push(crate::if_stmt!(
                    field.default_value_bin_expr(ctx, super::field::this_field_member),
                    crate::expr_stmt!(crate::assign_expr!(
                        PatOrExpr::Expr(Box::new(crate::member_expr_computed!(Expr::Ident(quote_ident!("json")), Expr::Ident(quote_ident!(format!("\"{}\"", field.json_key_name())))))),
                        value_expr
                    ))
                ))
            }
            
        }

        ClassMember::Method(ClassMethod {
            span: DUMMY_SP,
            accessibility: None,
            key: PropName::Ident(quote_ident!(format!("toJson_{}", index))),
            is_abstract: false,
            is_optional: false,
            is_override: false,
            is_static: false,
            function: Box::new(Function {
                body: Some(BlockStmt {
                    span: DUMMY_SP,
                    stmts: statements,
                }),
                decorators: vec![],
                is_async: false,
                is_generator: false,
                params: vec![Param {
                    span: DUMMY_SP,
                    decorators: vec![],
                    pat: swc_ecma_ast::Pat::Ident(swc_ecma_ast::BindingIdent {
                        id: quote_ident!("json"),
                        type_ann: Some(Box::new(crate::type_annotation!("object"))),
                    }),
                }   
                ],
                return_type: None,
                span: DUMMY_SP,
                type_params: None,
            }),
            kind: MethodKind::Method,
        })
    }

    pub(super) fn print_to_json(&self, ctx: &mut Context) -> Vec<ClassMember> {
        let mut statements = vec![Stmt::Decl(crate::const_decl!(
            "json: object",
            crate::new_expr!(Expr::Ident(quote_ident!("Object")))
        ))];
        let mut class_member_vec = Vec::new();
        let mut cur_field_vec = Vec::new();
        let mut cur_method_index = 0;

        let mut newst = |fields: &Vec<&FieldDescriptorProto>, index: i32| { 
            class_member_vec.push(self.print_to_json_inner(ctx, index, fields));
            // call inner method
            statements.push(crate::expr_stmt!(crate::call_expr!(
                crate::member_expr_bare!(Expr::Ident(quote_ident!("this")), format!("toJson_{}", index)),
                vec![
                    crate::expr_or_spread!(Expr::Ident(quote_ident!("json")))
                ]
            )));
        };


        for field in &self.field {
            cur_field_vec.push(field);
            if cur_field_vec.len() >= 30 {
                // add class member
                newst(&cur_field_vec, cur_method_index);
                cur_method_index += 1;
                cur_field_vec.clear();
            }
        }
    
        if !cur_field_vec.is_empty() {
            newst(&cur_field_vec, cur_method_index);
        }

        statements.push(crate::return_stmt!(quote_ident!("json").into()));

        let to_json_class_member = ClassMember::Method(ClassMethod {
            span: DUMMY_SP,
            accessibility: None,
            key: PropName::Ident(quote_ident!("toJson")),
            is_abstract: false,
            is_optional: false,
            is_override: false,
            is_static: false,
            function: Box::new(Function {
                body: Some(BlockStmt {
                    span: DUMMY_SP,
                    stmts: statements,
                }),
                decorators: vec![],
                is_async: false,
                is_generator: false,
                params: vec![],
                return_type: Some(Box::new(crate::type_annotation!("Object"))),
                span: DUMMY_SP,
                type_params: None,
            }),
            kind: MethodKind::Method,
        });
        class_member_vec.push(to_json_class_member);
        class_member_vec
    }

   fn print_from_json_inner(&self, ctx: &mut Context, index: i32, fields: &Vec<&FieldDescriptorProto>) -> ClassMember {
        let mut statements = vec![];
        for field in fields {
            let accessor_fn = if field.is_repeated() && !field.is_map(ctx) {
                super::field::static_field_member
            } else {
                super::field::bare_field_member
            };

            let mut value_expr = field.into_from_json_expr(ctx, accessor_fn);
            if field.is_enum() {
                if field.is_repeated() {
                    value_expr = Expr::Ident(quote_ident!("r"));
                } else {
                    value_expr = Expr::Ident(quote_ident!(format!("{}",field.name())));
                }
            } 

            if field.is_map(ctx) {
                value_expr = crate::call_expr!(
                    crate::member_expr_bare!(Expr::Ident(quote_ident!(field.name())), "forEach"),
                    vec![crate::expr_or_spread!(crate::arrow_func!(
                        vec![crate::pat_ident!(quote_ident!("value")), crate::pat_ident!(quote_ident!("key"))],
                        vec![
                            crate::expr_stmt!(crate::call_expr!(
                                crate::member_expr_bare!(
                                    crate::member_expr!("jsonMessage", format!("{}?", field.name())), "set"),
                                vec![
                                    crate::expr_or_spread!(Expr::TsNonNull(TsNonNullExpr {
                                        expr: Box::new(Expr::Ident(quote_ident!("key"))),
                                        span: DUMMY_SP
                                    })),
                                    crate::expr_or_spread!(Expr::TsNonNull(TsNonNullExpr {
                                        expr: Box::new(Expr::Ident(quote_ident!("value"))),
                                        span: DUMMY_SP
                                    })),
                                ]
                            ))
                        ]
                    ))]
                );
            } else if field.is_repeated() {
                value_expr = crate::call_expr!(
                    crate::member_expr_bare!(super::field::bare_field_member(&field), "map"),
                    vec![crate::expr_or_spread!(crate::arrow_func!(
                        vec![crate::pat_ident!(quote_ident!("r"))],
                        vec![
                            // field.value_check_stmt(ctx, super::field::static_field_member),
                            crate::return_stmt!(value_expr)
                        ]
                    ))]
                );
            }

            let mut stmts = vec![];

//             if !field.is_repeated() {
//                 stmts.push(field.value_check_stmt(ctx, accessor_fn))
//             }
            if field.has_oneof_index() {
                stmts.push(crate::if_stmt!(
                    crate::call_expr!(
                        crate::member_expr!("oneof", "has"),
                        vec![crate::expr_or_spread!(
                            crate::lit_num!(field.oneof_index()).into()
                        )]
                    ),
                    crate::throw_stmt!(crate::new_expr!(
                        quote_ident!("Error").into(),
                        vec![crate::expr_or_spread!(crate::lit_str!(format!(
                            "duplicate oneof field {}",
                            field.json_key_name()
                        ))
                        .into())]
                    ))
                ));
                stmts.push(crate::expr_stmt!(crate::call_expr!(
                    crate::member_expr!("oneof", "add"),
                    vec![crate::expr_or_spread!(
                        crate::lit_num!(field.oneof_index()).into()
                    )]
                )))
            }
            if field.is_map(ctx) {
                // nothing
                stmts.push(crate::expr_stmt!(value_expr))
            } else {
                stmts.push(crate::expr_stmt!(crate::assign_expr!(
                    PatOrExpr::Expr(Box::new(crate::member_expr!("jsonMessage", field.name()))),
                    value_expr
                )));
            }
           
            statements.push(Stmt::Decl(crate::const_decl!(
                format!("{}{}", field.name(), self.get_field_descriptor_str(ctx, &field)),
                crate::cond_expr!(
                    Expr::Ident(quote_ident!(format!("{}[\"{}\"] !== undefined", "json", field.json_key_name()))),
                    Expr::Ident(quote_ident!(format!("{}[\"{}\"]", "json", field.json_key_name()))),
                    Expr::Ident(quote_ident!(format!("{}[\"{}\"]", "json", field.name())))
                )
            )));
           
            statements.push(crate::if_stmt!(
                field.default_value_bin_expr_for_json(ctx, super::field::bare_field_member),
                crate::block_stmt!(stmts)
            ))
        }

        // statements.push(crate::return_stmt!(quote_ident!("jsonMessage").into()));
        
        ClassMember::Method(ClassMethod {
            span: DUMMY_SP,
            accessibility: None,
            key: PropName::Ident(quote_ident!(format!("{}_{}", "fromJson", index))),
            is_abstract: false,
            is_optional: false,
            is_override: false,
            is_static: true,
            function: Box::new(Function {
                body: Some(BlockStmt {
                    span: DUMMY_SP,
                    stmts: statements,
                }),
                decorators: vec![],
                is_async: false,
                is_generator: false,
                params: vec![
                    Param {
                        span: DUMMY_SP,
                        decorators: vec![],
                        pat: swc_ecma_ast::Pat::Ident(swc_ecma_ast::BindingIdent {
                           id: quote_ident!("json"),
                            type_ann: Some(Box::new(crate::type_annotation!("object"))),
                        }),
                    },
                    Param {
                        span: DUMMY_SP,
                        decorators: vec![],
                        pat: swc_ecma_ast::Pat::Ident(swc_ecma_ast::BindingIdent {
                           id: quote_ident!("jsonMessage"),
                            type_ann: Some(Box::new(crate::type_annotation!(crate::type_ref!(crate::entity_name_ident!(
                                quote_ident!(ctx.normalize_name(self.name()))
                            ))))),
                        }),
                    },
                ],
                return_type: None,
                span: DUMMY_SP,
                type_params: None,
            }),
            kind: MethodKind::Method,
        })
    }


    pub(super) fn print_from_json(&self, ctx: &mut Context) -> Vec<ClassMember> {
        let mut statements = vec![
            Stmt::Decl(crate::const_decl!(
                "jsonMessage",
                crate::new_expr!(Expr::Ident(quote_ident!(ctx.normalize_name(self.name()))))
            )),

        ];

        let mut has_oneof = false;
        for field in self.field.clone() {
            if field.has_oneof_index() {
                has_oneof = true;
                break;
            }
        }
        if has_oneof {
            statements.push(Stmt::Decl(crate::const_decl!(
                "oneof",
                crate::new_expr!(Expr::Ident(quote_ident!("Set")))
            )));
        }
        let mut class_member_vec: Vec<ClassMember> = [].to_vec();
        let mut cur_field_vec = Vec::new();
        let mut cur_method_index = 0;
        let mut newst = |fields: &Vec<&FieldDescriptorProto>, index: i32| {
            // add class member
            class_member_vec.push(self.print_from_json_inner(ctx, index, fields));
            // call inner method
            statements.push(
                crate::expr_stmt!(Expr::Ident(quote_ident!(format!("{}.fromJson_{}(json, jsonMessage)", ctx.normalize_name(self.name()), index)))));
        };
        for field in &self.field {
            cur_field_vec.push(field);
            if cur_field_vec.len() >= 30 {
                newst(&cur_field_vec, cur_method_index);
                cur_method_index +=1;
                cur_field_vec.clear();
            }
        }
        if !cur_field_vec.is_empty() {
            newst(&cur_field_vec, cur_method_index);
        }

        statements.push(crate::return_stmt!(quote_ident!("jsonMessage").into()));
        
        let from_json_class_member = ClassMember::Method(ClassMethod {
            span: DUMMY_SP,
            accessibility: None,
            key: PropName::Ident(quote_ident!("fromJson")),
            is_abstract: false,
            is_optional: false,
            is_override: false,
            is_static: true,
            function: Box::new(Function {
                body: Some(BlockStmt {
                    span: DUMMY_SP,
                    stmts: statements,
                }),
                decorators: vec![],
                is_async: false,
                is_generator: false,
                params: vec![Param {
                    span: DUMMY_SP,
                    decorators: vec![],
                    pat: swc_ecma_ast::Pat::Ident(swc_ecma_ast::BindingIdent {
                        id: quote_ident!("json"),
                        type_ann: Some(Box::new(crate::type_annotation!("object"))),
                    }),
                }],
                return_type: Some(Box::new(crate::type_annotation!(crate::type_ref!(crate::entity_name_ident!(
                    quote_ident!(ctx.normalize_name(self.name()))
                ))))),
                span: DUMMY_SP,
                type_params: None,
            }),
            kind: MethodKind::Method,
        });
        class_member_vec.push(from_json_class_member);
        class_member_vec 
    }
}
