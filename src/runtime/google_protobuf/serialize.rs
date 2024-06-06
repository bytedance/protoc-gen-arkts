 /**
  * Copyright 2024 ByteDance and/or its affiliates
  *
  * Original Files：protoc-gen-ts (https://github.com/thesayyn/protoc-gen-ts)
  * Copyright (c) 2024 Sahin Yort
  * SPDX-License-Identifier: MIT 
 */

use super::GooglePBRuntime;
use crate::common::field;
use crate::{context::Context, descriptor};

use std::vec;
use swc_common::DUMMY_SP;
use swc_ecma_ast::{
    BindingIdent, BlockStmt, Expr, ForHead, ForOfStmt, Stmt, TsNonNullExpr, VarDecl,
};
use swc_ecma_utils::quote_ident;

impl GooglePBRuntime {
    // const MAX_UINT32 = BigInt(2 ** 32);
    // bw.writePackedSplitFixed64(
    //   40,
    //   this.repeated_sfixed64,
    //   (i) => Number(i & 4294967295n),
    //   (i) => Number((i >> 32n) & 4294967295n)
    // );
    pub fn serialize_workaround_sfixed64_field_stmt(
        &self,
        field: &descriptor::FieldDescriptorProto,
        field_accessor: field::FieldAccessorFn,
    ) -> Stmt {
        crate::expr_stmt!(crate::call_expr!(
            crate::member_expr!("bw", "writePackedSplitFixed64"),
            vec![
                crate::expr_or_spread!(crate::lit_num!(field.number()).into()),
                crate::expr_or_spread!(field_accessor(field)),
                crate::expr_or_spread!(quote_ident!("(i) => Number(i & 4294967295n)").into()),
                crate::expr_or_spread!(
                    quote_ident!("(i) => Number((i >> 32n) & 4294967295n)").into()
                ),
            ]
        ))
    }

    pub fn serialize_primitive_field_stmt(
        &self,
        ctx: &mut Context,
        field: &descriptor::FieldDescriptorProto,
        field_accessor: field::FieldAccessorFn,
        access_normalizer: Option<field::AccessNormalizerFn>,
    ) -> Stmt {
        let mut access_expr = field_accessor(field);
        if let Some(an) = access_normalizer {
            access_expr = an(&access_expr)
        }

        let mut field_params = crate::expr_or_spread!(access_expr);
        if ctx.options.with_sendable && field.is_bytes() {
            field_params = crate::expr_or_spread!(
                crate::call_expr!(crate::member_expr!("Uint8Array", "from"), 
                    vec![
                        field_params
                    ]
                )
            )
        }
        crate::expr_stmt!(crate::call_expr!(
            crate::member_expr!("bw", self.rw_function_name("write", ctx, field)),
            vec![
                crate::expr_or_spread!(crate::lit_num!(field.number()).into()),
                field_params,
            ]
        ))
    }

    pub fn serialize_message_field_stmt(
        &self,
        field: &descriptor::FieldDescriptorProto,
        field_accessor: field::FieldAccessorFn,
    ) -> Stmt {
        crate::expr_stmt!(crate::call_expr!(
            crate::member_expr!("bw", "writeBytes"),
            vec![
                crate::expr_or_spread!(crate::lit_num!(field.number()).into()),
                crate::expr_or_spread!(crate::call_expr!(crate::member_expr_bare!(
                    Expr::TsNonNull(TsNonNullExpr {
                        expr: Box::new(field_accessor(field)),
                        span: DUMMY_SP
                    }),
                    "toBinary"
                ))),
            ]
        ))
    }

    fn serialize_map_field_stmt(
        &self,
        ctx: &mut Context,
        field: &descriptor::FieldDescriptorProto,
    ) -> Stmt {
        let descriptor = ctx
            .get_map_type(field.type_name())
            .expect(format!("can not find the map type {}", field.type_name()).as_str());

        let mut stmts = vec![crate::expr_stmt!(crate::call_expr!(
            crate::member_expr!("bw", "beginSubMessage"),
            vec![crate::expr_or_spread!(
                crate::lit_num!(field.number()).into()
            )]
        ))];

        stmts.push(crate::expr_stmt!(Expr::Ident(quote_ident!("let key = entry[0]"))));
        stmts.push(crate::expr_stmt!(Expr::Ident(quote_ident!("let value = entry[1]"))));

        stmts.append(&mut self.serialize_setup_inner(
            ctx,
            &descriptor,
            field::bare_field_member,
            false,
            false,
        ));

        stmts.push(crate::expr_stmt!(crate::call_expr!(
            crate::member_expr!("bw", "endSubMessage"),
            vec![]
            // vec![crate::expr_or_spread!(Expr::Lit(crate::lit_num!(
            //     field.number()
            // )))]
        )));

        Stmt::ForOf(ForOfStmt {
            is_await: false,
            left: ForHead::Pat(Box::new(crate::pat_ident!(quote_ident!("let entry")))),
            right: Box::new(crate::member_expr_bare!(crate::member_expr!("this", field.name()), "entries()")),
            body: Box::new(Stmt::Block(BlockStmt {
                span: DUMMY_SP,
                stmts,
            })),
            span: DUMMY_SP,
        })
    }
}

impl GooglePBRuntime {
    pub(super) fn serialize_setup_inner(
        &self,
        ctx: &mut Context,
        descriptor: &descriptor::DescriptorProto,
        accessor: field::FieldAccessorFn,
        create_bw: bool,
        prevent_defaults: bool,
    ) -> Vec<Stmt> {
        let mut stmts = vec![];

        if create_bw {
            ctx.get_protobuf_import(&ctx.options.runtime_package);
            let bw_decl_init = crate::new_expr!(Expr::Ident(quote_ident!("BinaryWriter")));
            let bw_decl = Stmt::Decl(
                crate::const_decl!(
                    format!("{}: BinaryWriter", "bw"), bw_decl_init));
            stmts.push(bw_decl)
        }

        for field in &descriptor.field {
            let field_accessor = if descriptor.options.map_entry() {
                accessor
            } else {
                field.into_accessor(ctx)
            };

            let access_normalizer: Option<field::AccessNormalizerFn> =
                if field.is_bigint() && field.is_packed(&ctx) {
                    Some(field::map_to_string_normalizer)
                } else if field.is_bigint() {
                    Some(field::to_string_normalizer)
                } else {
                    None
                };

            let mut field_stmt: Stmt;

            if field.is_message() {
                field_stmt = self.serialize_message_field_stmt(field, field_accessor)
            } else if field.type_() == descriptor::field_descriptor_proto::Type::TYPE_SFIXED64
                && field.is_packed(ctx)
            {
                field_stmt = self.serialize_workaround_sfixed64_field_stmt(field, field_accessor)
            } else {
                field_stmt = self.serialize_primitive_field_stmt(
                    ctx,
                    field,
                    field_accessor,
                    access_normalizer,
                )
            };

            if field.is_map(ctx) {
                field_stmt = self.serialize_map_field_stmt(ctx, field)
            } else if field.is_repeated() && !field.is_packed(ctx) {
                if ctx.options.with_sendable {
                    field_stmt = crate::expr_stmt!(crate::call_expr!(
                        crate::member_expr_bare!(super::field::this_field_member(field), "forEach"),
                        vec![crate::expr_or_spread!(crate::arrow_func!(
                            vec![crate::pat_ident!(quote_ident!(field.name()))],
                            vec![
                                field_stmt
                            ]
                        ))]
                    ))

                } else {
                    field_stmt = Stmt::ForOf(ForOfStmt {
                        is_await: false,
                        left: ForHead::VarDecl(Box::new(crate::const_decl_uinit!(field.name()))),
                        right: Box::new(accessor(field)),
                        body: Box::new(Stmt::Block(BlockStmt {
                            span: DUMMY_SP,
                            stmts: vec![field_stmt],
                        })),
                        span: DUMMY_SP,
                    })
                }
            }

            if prevent_defaults {
                stmts.push(crate::if_stmt!(
                    field.default_value_bin_expr(ctx, accessor),
                    Stmt::Block(BlockStmt {
                        span: DUMMY_SP,
                        stmts: vec![field_stmt]
                    })
                ));
            } else {
                stmts.push(field_stmt);
            }
        }

        // serialize unknown fields
        if create_bw {
            // stmts.push(Stmt::ForOf(ForOfStmt {
            //     is_await: false,
            //     left: ForHead::VarDecl(Box::new(crate::const_decl_uinit!("uf"))),
            //     right: Box::new(crate::member_expr!("this", "#unknown_fields")),
            //     body: Box::new(Stmt::Block(BlockStmt {
            //         span: DUMMY_SP,
            //         stmts: vec![
            //             crate::expr_stmt!(crate::call_expr!(
            //                 crate::member_expr!("bw", "writeFieldHeader_"),
            //                 vec![
            //                     crate::expr_or_spread!(crate::member_expr!("uf", "no")),
            //                     crate::expr_or_spread!(crate::member_expr!("uf", "wireType"))
            //                 ]
            //             )),
            //             crate::expr_stmt!(crate::call_expr!(
            //                 crate::member_expr!("bw", "appendUint8Array_"),
            //                 vec![crate::expr_or_spread!(crate::member_expr!("uf", "data")),]
            //             )),
            //         ],
            //     })),
            //     span: DUMMY_SP,
            // }));
        }

        stmts
    }
}
