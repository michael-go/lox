mod chunk;
mod value;
mod vm;

#[macro_use]
extern crate num_derive;

fn main() -> Result<(), vm::LoxError> {
    let mut chunk = chunk::Chunk::new();

    let constant = chunk.add_constant(1.2);
    chunk.write_chunk(chunk::OpCode::Constant.u8(), 123);
    chunk.write_chunk(constant, 123);
    chunk.write_chunk(chunk::OpCode::Negate.u8(), 123);
    chunk.write_chunk(chunk::OpCode::Return.u8(), 200);

    // chunk.dissasemble("test chunk");

    println!("");

    let mut vm = vm::VM::new(chunk);
    // TODO: control options by args / env
    if let Err(e) = vm.interpret(vm::Options {
        trace_execution: true,
    }) {
        println!("Error: {:?}", e);
        return Err(e);
    }
    return Ok(());
}
