use std::{
    collections::VecDeque,
    ops::{Add, Div, Mul, Neg, Rem, Sub},
};

use bevy::{
    prelude::{default, HandleUntyped, Plugin, Shader, UVec4, Vec4},
    reflect::{FromReflect, Reflect, TypeUuid},
    render::render_resource::ShaderType,
};

use crate::load_internal_asset;

pub const STACK_MACHINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 12198237481790981807);

pub struct StackMachinePlugin;

impl Plugin for StackMachinePlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        StackMachineUniform::assert_uniform_compat();

        load_internal_asset!(
            app,
            STACK_MACHINE_HANDLE,
            "stack_machine.wgsl",
            Shader,
            Shader::from_wgsl
        );
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Opcode<T> {
    Nop,
    Literal(T),
    Context(u32),
    Neg,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Abs,
    Min,
    Max,
}

impl<T> From<Opcode<T>> for u32 {
    fn from(value: Opcode<T>) -> Self {
        match value {
            Opcode::Nop => 0,
            Opcode::Literal(_) => 1,
            Opcode::Context(_) => 2,
            Opcode::Neg => 3,
            Opcode::Add => 4,
            Opcode::Sub => 5,
            Opcode::Mul => 6,
            Opcode::Div => 7,
            Opcode::Mod => 8,
            Opcode::Abs => 9,
            Opcode::Min => 10,
            Opcode::Max => 11,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PCode<T>(Vec<Opcode<T>>);

impl<T> IntoIterator for PCode<T> {
    type Item = Opcode<T>;

    type IntoIter = std::vec::IntoIter<Opcode<T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<T> FromIterator<Opcode<T>> for PCode<T> {
    fn from_iter<U: IntoIterator<Item = Opcode<T>>>(iter: U) -> Self {
        PCode(iter.into_iter().collect())
    }
}

#[derive(Debug, Default, Copy, Clone, ShaderType, Reflect, FromReflect)]
pub struct StackMachineUniform {
    pub pcode: [UVec4; 16],
    pub push_buf: [Vec4; 16],
    pub context_idx_buf: [UVec4; 16],
}

impl From<PCode<f32>> for StackMachineUniform {
    fn from(value: PCode<f32>) -> Self {
        let mut pcode = VecDeque::default();
        let mut push_buf = VecDeque::default();
        let mut context_idx_buf = VecDeque::default();

        for opcode in value {
            let c: u32 = opcode.into();
            pcode.push_back(c);

            if let Opcode::Literal(t) = opcode {
                push_buf.push_back(t);
            }

            if let Opcode::Context(i) = opcode {
                context_idx_buf.push_back(i);
            }
        }

        let mut uniform = StackMachineUniform::default();

        for i in 0..16 {
            for u in 0..4 {
                if let Some(opcode) = pcode.pop_front() {
                    uniform.pcode[i][u] = opcode;
                };

                if let Some(push) = push_buf.pop_front() {
                    uniform.push_buf[i][u] = push;
                }

                if let Some(idx) = context_idx_buf.pop_front() {
                    uniform.context_idx_buf[i][u] = idx;
                }
            }
        }

        uniform
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StackMachine<T> {
    stack: [T; 16],
    stack_pointer: isize,
    pcode: Vec<Opcode<T>>,
}

impl<T> Default for StackMachine<T>
where
    T: Default,
{
    fn default() -> Self {
        StackMachine {
            stack: default(),
            stack_pointer: -1,
            pcode: default(),
        }
    }
}

impl StackMachine<f32> {
    fn evaluate(&mut self, op: Opcode<f32>) {
        println!("{op:#?}");
        match op {
            Opcode::Nop => todo!(),
            Opcode::Literal(t) => self.push(t),
            Opcode::Context(i) => todo!(),
            Opcode::Neg => {
                let t = self.read();
                *self.write() = Neg::neg(t);
            }
            Opcode::Add => {
                let rhs = self.pop();
                let lhs = self.read();
                *self.write() = Add::add(lhs, rhs);
            }
            Opcode::Sub => {
                let rhs = self.pop();
                let lhs = self.read();
                *self.write() = Sub::sub(lhs, rhs);
            }
            Opcode::Mul => {
                let rhs = self.pop();
                let lhs = self.read();
                *self.write() = Mul::mul(lhs, rhs);
            }
            Opcode::Div => {
                let rhs = self.pop();
                let lhs = self.read();
                *self.write() = Div::div(lhs, rhs);
            }
            Opcode::Mod => {
                let rhs = self.pop();
                let lhs = self.read();
                *self.write() = Rem::rem(lhs, rhs);
            }
            Opcode::Abs => {
                let t = self.read();
                *self.write() = t.abs();
            }
            Opcode::Min => {
                let rhs = self.pop();
                let lhs = self.read();
                *self.write() = lhs.min(rhs);
            }
            Opcode::Max => {
                let rhs = self.pop();
                let lhs = self.read();
                *self.write() = lhs.max(rhs);
            }
        }
        println!("{:#?}", self.stack);
    }

    fn tick(&mut self) {
        let op = self.pcode.remove(0);
        self.evaluate(op);
    }

    fn run(&mut self) -> f32 {
        for op in self.pcode.drain(..).collect::<Vec<_>>() {
            self.evaluate(op);
        }

        self.pop()
    }
}

impl<T> StackMachine<T>
where
    T: Default + Clone,
{
    fn read(&self) -> T {
        self.stack[self.stack_pointer as usize].clone()
    }

    fn write(&mut self) -> &mut T {
        &mut self.stack[self.stack_pointer as usize]
    }

    fn push(&mut self, t: T) {
        self.stack_pointer += 1;
        *self.write() = t;
    }

    fn pop(&mut self) -> T {
        let t = self.read();
        *self.write() = default();
        self.stack_pointer -= 1;
        t
    }

    fn pop2(&mut self) -> (T, T) {
        (self.pop(), self.pop())
    }
}

#[test]
fn test_stack_machine() {
    let mut sm = StackMachine {
        pcode: vec![
            Opcode::Literal(2.0),
            Opcode::Literal(4.0),
            Opcode::Mul,
            Opcode::Literal(0.5),
            Opcode::Div,
        ],
        ..default()
    };

    println!("{:#?}", sm.stack);

    let result = sm.run();
    println!("Result: {result:}");
}
