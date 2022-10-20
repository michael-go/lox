mod chunk;
mod value;
mod vm;

#[macro_use]
extern crate num_derive;

use chunk::OpCode;
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    disassemble: bool,
    #[arg(short, long)]
    trace_execution: bool,
}

fn main() -> Result<(), vm::LoxError> {
    let args = Args::parse();

    let mut chunk = chunk::Chunk::new();
    let constant = chunk.add_constant(1.2);
    chunk.write_chunk(chunk::OpCode::Constant.u8(), 123);
    chunk.write_chunk(constant, 123);
    let constant2 = chunk.add_constant(3.4);
    chunk.write_chunk(OpCode::Constant.u8(), 123);
    chunk.write_chunk(constant2, 123);
    chunk.write_chunk(OpCode::Add.u8(), 123);
    let constant3 = chunk.add_constant(5.6);
    chunk.write_chunk(OpCode::Constant.u8(), 123);
    chunk.write_chunk(constant3, 123);
    chunk.write_chunk(OpCode::Divide.u8(), 123);
    chunk.write_chunk(chunk::OpCode::Negate.u8(), 123);
    chunk.write_chunk(chunk::OpCode::Return.u8(), 200);

    if args.disassemble {
        chunk.dissasemble("test chunk");
        println!("");
    } else {
        let mut vm = vm::VM::new(chunk);
        if let Err(e) = vm.interpret(vm::Options {
            trace_execution: args.trace_execution,
        }) {
            println!("Error: {:?}", e);
            return Err(e);
        }
    }
    return Ok(());
}
