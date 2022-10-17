mod chunk;

fn main() {
    let mut chunk = chunk::Chunk::new();
    
    let constant = chunk.add_constant(1.2);
    chunk.write_chunk(chunk::OpCode::OpConstant.u8(), 123);
    chunk.write_chunk(constant, 123);
    chunk.write_chunk(chunk::OpCode::OpReturn.u8(), 123);
    chunk.write_chunk(137, 234);
    
    chunk.dissasemble("test chunk");
}
