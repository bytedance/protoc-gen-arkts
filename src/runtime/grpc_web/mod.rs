 /**
  * Copyright 2024 ByteDance and/or its affiliates
  *
  * Original Files：protoc-gen-ts (https://github.com/thesayyn/protoc-gen-ts)
  * Copyright (c) 2024 Sahin Yort
  * SPDX-License-Identifier: MIT 
 */

use swc_ecma_ast::ClassMember;

use crate::{context::Context, descriptor::{ServiceDescriptorProto, MethodDescriptorProto}};

use super::GrpcRuntime;

#[derive(Clone)]
pub struct GrpcWebRuntime {}

impl GrpcWebRuntime {
    pub fn new() -> Self {
        GrpcWebRuntime {}
    }
}

impl GrpcRuntime for GrpcWebRuntime {
    fn print_setup(&self, ctx: &mut Context) -> Vec<ClassMember> {
        // let mut members = self.print_props(ctx);
        // members.push(self.print_constructor(ctx));
        // members
        [].to_vec()
    }

    fn print_method(
        &self,
        ctx: &mut Context,
        method: &MethodDescriptorProto,
        service: &ServiceDescriptorProto
    ) -> Vec<ClassMember> {
        vec![
            // self.print_descriptor(ctx, method, service),
            // self.print_method(ctx, method, service)
        ]
    }
}

mod method;
mod setup;