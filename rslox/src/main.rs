mod chunk;
mod compiler;
mod scanner;
mod value;
mod vm;

#[macro_use]
extern crate num_derive;

use anyhow::Result;
use clap::Parser;
use std::io::{self, Write};

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    disassemble: bool,
    #[arg(short, long)]
    trace_execution: bool,

    lox_file: Option<String>,
}

fn repl(options: vm::Options) -> Result<()> {
    let mut vm = vm::VM::new(options);
    loop {
        print!("> ");
        io::stdout().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        let result = vm.interpret(&line);
        match result {
            Ok(_) => (),
            Err(e) => println!("{}", e),
        }
    }
}

fn run_file(path: &str, options: vm::Options) -> Result<()> {
    let mut vm = vm::VM::new(options);

    let source = std::fs::read_to_string(path)?;
    let _ = vm.interpret(&source);
    Ok(())
}

fn main() -> Result<()> {
    std::env::set_var("RUST_BACKTRACE", "full");

    let args = Args::parse();

    let options = vm::Options {
        trace_execution: args.trace_execution,
    };

    // TODO: handle errors
    if let Some(lox_file) = args.lox_file {
        run_file(&lox_file, options)
    } else {
        repl(options)
    }
}
