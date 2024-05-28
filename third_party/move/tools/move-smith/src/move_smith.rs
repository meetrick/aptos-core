// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ast::*,
    config::Config,
    names::{IdentifierPool, IdentifierType, Scope},
    types::{Type, TypePool},
};
use arbitrary::{Arbitrary, Result, Unstructured};
use num_bigint::BigUint;

pub struct MoveSmith {
    pub config: Config,

    // The output code
    pub modules: Vec<Module>,

    // Bookkeeping
    pub id_pool: IdentifierPool,
    pub type_pool: TypePool,
}

impl Default for MoveSmith {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl MoveSmith {
    pub fn new(config: Config) -> Self {
        Self {
            modules: Vec::new(),
            config,
            id_pool: IdentifierPool::new(),
            type_pool: TypePool::new(),
        }
    }

    pub fn generate_module(&mut self, u: &mut Unstructured) -> Result<Module> {
        let (name, scope) = self.id_pool.next_identifier(IdentifierType::Module, &None);

        let len = u.int_in_range(1..=self.config.max_members_in_module)?;
        let mut members = Vec::new();
        for _ in 0..len {
            members.push(self.generate_module_member(u, &scope)?);
        }

        Ok(Module { name, members })
    }

    fn generate_module_member(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<ModuleMember> {
        match u.int_in_range(0..=0)? {
            0 => Ok(ModuleMember::Function(
                self.generate_function(u, parent_scope)?,
            )),
            _ => panic!("Invalid module member type"),
        }
    }

    fn generate_function(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Function> {
        let (name, scope) = self
            .id_pool
            .next_identifier(IdentifierType::Function, parent_scope);
        Ok(Function {
            name,
            body: self.generate_function_body(u, &scope)?,
        })
    }

    fn generate_function_body(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<FunctionBody> {
        let len = u.int_in_range(0..=self.config.max_stmt_in_func)?;
        let mut stmts = Vec::new();
        for _ in 0..len {
            stmts.push(self.generate_statement(u, parent_scope)?);
        }
        Ok(FunctionBody { stmts })
    }

    fn generate_statement(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Statement> {
        match u.int_in_range(0..=1)? {
            0 => Ok(Statement::Decl(self.generate_decalration(u, parent_scope)?)),
            1 => Ok(Statement::Expr(self.generate_expression(u, parent_scope)?)),
            _ => panic!("Invalid statement type"),
        }
    }

    fn generate_decalration(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Declaration> {
        let (name, _) = self
            .id_pool
            .next_identifier(IdentifierType::Var, parent_scope);

        let typ = self.type_pool.random_basic_type(u)?;
        // let value = match bool::arbitrary(u)? {
        //     true => Some(self.generate_expression_of_type(u, parent_scope, &typ)?),
        //     false => None,
        // };
        // TODO: disabled declaration without value for now, need to keep track of initialization
        let value = Some(self.generate_expression_of_type(u, parent_scope, &typ)?);
        self.type_pool.insert(&name, &typ);
        Ok(Declaration { typ, name, value })
    }

    fn generate_expression(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
    ) -> Result<Expression> {
        let expr = loop {
            match u.int_in_range(0..=1)? {
                0 => {
                    break Expression::NumberLiteral(self.generate_number_literal(
                        u,
                        parent_scope,
                        None,
                    )?)
                },
                1 => {
                    if let Some(ident) = self.id_pool.random_existing_identifier(
                        u,
                        parent_scope,
                        Some(IdentifierType::Var),
                    )? {
                        break Expression::Variable(ident);
                    }
                },
                _ => panic!("Invalid expression type"),
            }
        };
        Ok(expr)
    }

    fn generate_expression_of_type(
        &mut self,
        u: &mut Unstructured,
        parent_scope: &Scope,
        typ: &Type,
    ) -> Result<Expression> {
        // Store candidate expressions for the given type
        let mut choices: Vec<Expression> = Vec::new();

        // Directly generate a value for basic types
        if typ.is_basic_type() {
            let candidate = match typ {
                Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::U256 => {
                    Expression::NumberLiteral(self.generate_number_literal(
                        u,
                        parent_scope,
                        Some(typ),
                    )?)
                },
                _ => unimplemented!(),
            };
            choices.push(candidate);
        }

        // Access identifier with the given type
        let ident_of_typ = self.type_pool.get_identifiers_of_type(typ);
        let in_scope = self.id_pool.filter_under_scope(&ident_of_typ, parent_scope);

        // TODO: select from many?
        if !in_scope.is_empty() {
            let candidate = u.choose(&in_scope)?.clone();
            choices.push(Expression::Variable(candidate));
        }

        // TODO: call functions with the given type

        Ok(u.choose(&choices)?.clone())
    }

    /// Generate a random numerical literal.
    /// If the `typ` is `None`, a random type will be chosen.
    /// If the `typ` is `Some(Type::{U8, ..., U256})`, a literal of the given type will be used.
    fn generate_number_literal(
        &mut self,
        u: &mut Unstructured,
        _parent_scope: &Scope,
        typ: Option<&Type>,
    ) -> Result<NumberLiteral> {
        let idx = match typ {
            Some(t) => match t {
                Type::U8 => 0,
                Type::U16 => 1,
                Type::U32 => 2,
                Type::U64 => 3,
                Type::U128 => 4,
                Type::U256 => 5,
                _ => panic!("Invalid number literal type"),
            },
            None => u.int_in_range(0..=5)?,
        };

        Ok(match idx {
            0 => NumberLiteral {
                value: BigUint::from(u8::arbitrary(u)?),
                typ: Type::U8,
            },
            1 => NumberLiteral {
                value: BigUint::from(u16::arbitrary(u)?),
                typ: Type::U16,
            },
            2 => NumberLiteral {
                value: BigUint::from(u32::arbitrary(u)?),
                typ: Type::U32,
            },
            3 => NumberLiteral {
                value: BigUint::from(u64::arbitrary(u)?),
                typ: Type::U64,
            },
            4 => NumberLiteral {
                value: BigUint::from(u128::arbitrary(u)?),
                typ: Type::U128,
            },
            5 => NumberLiteral {
                value: BigUint::from_bytes_be(u.bytes(32)?),
                typ: Type::U256,
            },
            _ => panic!("Invalid number literal type"),
        })
    }
}
