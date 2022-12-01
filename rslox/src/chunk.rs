use anyhow::Result;
use num_traits::FromPrimitive;
use std::collections::HashMap;

use crate::object::*;
use crate::value::Value;

#[derive(Clone, Copy, FromPrimitive)]
pub enum OpCode {
    Constant = 0,
    Nil,
    True,
    False,
    Pop,
    GetLocal,
    SetLocal,
    GetGlobal,
    DefineGlobal,
    SetGlobal,
    GetProperty,
    SetProperty,
    GetUpvalue,
    SetUpvalue,
    Equal,
    Greater,
    Less,
    Add,
    Subtract,
    Multiply,
    Divide,
    Negate,
    Not,
    Print,
    Jump,
    JumpIfFalse,
    Loop,
    Call,
    Closure,
    CloseUpvalue,
    Return,
    Class,
}

impl OpCode {
    pub fn u8(&self) -> u8 {
        *self as u8
    }
}

#[derive(Clone, PartialEq)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
    pub lines: HashMap<usize, usize>,
}

impl Chunk {
    pub fn new() -> Chunk {
        Chunk {
            code: Vec::new(),
            constants: Vec::new(),
            lines: HashMap::new(),
        }
    }

    pub fn write_chunk(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.insert(self.code.len() - 1, line);
    }

    pub fn add_constant(&mut self, value: Value) -> Result<u8> {
        if self.constants.len() >= u8::MAX as usize {
            return Err(anyhow::anyhow!("Too many constants in one chunk."));
        }

        self.constants.push(value);
        Ok((self.constants.len() - 1) as u8)
    }

    pub fn dissasemble(&self, name: &str) {
        println!("== {} ==", name);
        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }

    // TODO: move to debug.rs
    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);

        if offset > 0 && self.lines.get(&offset) == self.lines.get(&(offset - 1)) {
            print!("   | ");
        } else {
            let line = self.lines.get(&offset).unwrap();
            print!("{:04} ", line);
        }

        let instruction = self.code[offset];
        match FromPrimitive::from_u8(instruction) {
            Some(OpCode::Constant) => self.dissasemble_constant_instruction("Constant", offset),
            Some(OpCode::Nil) => self.dissasemble_simple_instruction("Nil", offset),
            Some(OpCode::True) => self.dissasemble_simple_instruction("True", offset),
            Some(OpCode::False) => self.dissasemble_simple_instruction("False", offset),
            Some(OpCode::Pop) => self.dissasemble_simple_instruction("Pop", offset),
            Some(OpCode::GetLocal) => self.dissasemble_byte_instruction("GetLocal", offset),
            Some(OpCode::SetLocal) => self.dissasemble_byte_instruction("SetLocal", offset),
            Some(OpCode::GetGlobal) => self.dissasemble_constant_instruction("GetGlobal", offset),
            Some(OpCode::DefineGlobal) => {
                self.dissasemble_constant_instruction("DefineGlobal", offset)
            }
            Some(OpCode::SetGlobal) => self.dissasemble_constant_instruction("SetGlobal", offset),
            Some(OpCode::GetProperty) => {
                self.dissasemble_constant_instruction("GetProperty", offset)
            }
            Some(OpCode::SetProperty) => {
                self.dissasemble_constant_instruction("SetProperty", offset)
            }
            Some(OpCode::GetUpvalue) => self.dissasemble_byte_instruction("GetUpvalue", offset),
            Some(OpCode::SetUpvalue) => self.dissasemble_byte_instruction("SetUpvalue", offset),
            Some(OpCode::Equal) => self.dissasemble_simple_instruction("Equal", offset),
            Some(OpCode::Greater) => self.dissasemble_simple_instruction("Greater", offset),
            Some(OpCode::Less) => self.dissasemble_simple_instruction("Less", offset),
            Some(OpCode::Add) => self.dissasemble_simple_instruction("Add", offset),
            Some(OpCode::Subtract) => self.dissasemble_simple_instruction("Subtract", offset),
            Some(OpCode::Multiply) => self.dissasemble_simple_instruction("Multiply", offset),
            Some(OpCode::Divide) => self.dissasemble_simple_instruction("Divide", offset),
            Some(OpCode::Negate) => self.dissasemble_simple_instruction("Negate", offset),
            Some(OpCode::Not) => self.dissasemble_simple_instruction("Not", offset),
            Some(OpCode::Print) => self.dissasemble_simple_instruction("Print", offset),
            Some(OpCode::Jump) => self.dissasemble_jump_instruction("Jump", 1, offset),
            Some(OpCode::JumpIfFalse) => {
                self.dissasemble_jump_instruction("JumpIfFalse", 1, offset)
            }
            Some(OpCode::Loop) => self.dissasemble_jump_instruction("Loop", -1, offset),
            Some(OpCode::Call) => self.dissasemble_byte_instruction("Call", offset),
            Some(OpCode::Closure) => {
                let constant = self.code[offset + 1];
                print!("{:16} {:04} ", "Closure", constant);
                let value = &self.constants[constant as usize];
                println!("{}", value);

                let mut next_offset = offset + 2;
                if let Value::Obj(obj) = value {
                    let function = obj.downcast_ref::<Function>().unwrap();
                    for _ in 0..function.upvalue_count {
                        let is_local = self.code[offset];
                        let is_local_str = if is_local == 1 { "local" } else { "upvalue" };
                        let index = self.code[offset + 1];
                        println!(
                            "{:04}      |                     {:04} {}",
                            offset, is_local_str, index
                        );
                        next_offset += 2;
                    }
                } else {
                    panic!("Expected function value.");
                }

                next_offset
            }
            Some(OpCode::CloseUpvalue) => {
                self.dissasemble_simple_instruction("CloseUpvalue", offset)
            }
            Some(OpCode::Return) => self.dissasemble_simple_instruction("Return", offset),
            Some(OpCode::Class) => self.dissasemble_constant_instruction("Class", offset),
            None => {
                println!("Unknown opcode {}", instruction);
                offset + 1
            }
        }
    }

    fn dissasemble_simple_instruction(&self, name: &str, offset: usize) -> usize {
        println!("{}", name);
        offset + 1
    }

    fn dissasemble_constant_instruction(&self, arg: &str, offset: usize) -> usize {
        let constant = self.code[offset + 1];
        print!("{:16} {:4} '", arg, constant);
        println!("{}", self.constants.get(constant as usize).unwrap());
        offset + 2
    }

    fn dissasemble_byte_instruction(&self, name: &str, offset: usize) -> usize {
        let byte = self.code[offset + 1];
        println!("{:16} {:4}", name, byte);
        offset + 2
    }

    fn dissasemble_jump_instruction(&self, name: &str, sign: i32, offset: usize) -> usize {
        let mut jump = (self.code[offset + 1] as u16) << 8;
        jump |= self.code[offset + 2] as u16;
        println!(
            "{:16} {:4} -> {}\n",
            name,
            offset,
            offset as i32 + 3 + (sign * (jump as i32))
        );
        offset + 3
    }
}
