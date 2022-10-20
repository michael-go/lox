use std::collections::HashMap;
use crate::value::Value;

pub enum OpCode {
    Constant = 0,
    Negate,
    Return,
}

impl OpCode {
    pub fn u8(&self) -> u8 {
        *self as u8
    }
}


pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: Vec<Value>,
    lines: HashMap<usize, usize>,
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
    
    pub fn add_constant(&mut self, value: Value) -> u8 {
        // TODO: ensure we don't allow more than u8::MAX constants
        self.constants.push(value);
        (self.constants.len() - 1) as u8
    }

    pub fn dissasemble(&self, name: &str) {
        println!("== {} ==", name);
        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }

    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);
        
        if offset > 0 && self.lines.get(&offset) == self.lines.get(&(offset - 1)) {
            print!("   | ");
        } else {
            let line = self.lines.get(&offset).unwrap();
            print!("{:04} ", line);
        }

        let instruction = self.code[offset];
        match instruction {
            // TODO: maybe instead add OpCode::from_u8() and match on Result/Error
            op if op == OpCode::Constant.u8() => {
                self.dissasemble_constant_instruction("OpConstant", offset)
            }
            op if op == OpCode::Negate.u8() => {
                self.dissasemble_simple_instruction("Negate", offset)
            }
            op if op == OpCode::Return.u8() => {
                self.dissasemble_simple_instruction("OpReturn", offset)
            }
            _ => {
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
        self.print_value(self.constants.get(constant as usize).unwrap());
        print!("'\n");
        offset + 2
    }
    
    fn print_value(&self, value: &Value) {
        print!("{}", value);
    }
}
