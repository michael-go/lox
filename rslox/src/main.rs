#![feature(linked_list_cursors)]

mod chunk;
mod compiler;
mod object;
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
    disassemble: bool, // TODO: forgot to handle this
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

    match std::fs::read_to_string(path) {
        Ok(source) => vm.interpret(&source),
        Err(e) => {
            println!("Error reading file {}: {}", path, e);
            Err(e.into())
        }
    }
}

fn main() {
    let args = Args::parse();

    let options = vm::Options {
        trace_execution: args.trace_execution,
    };

    let res: Result<()>;
    if let Some(lox_file) = args.lox_file {
        res = run_file(&lox_file, options)
    } else {
        res = repl(options)
    }

    if res.is_err() {
        match res.unwrap_err().downcast::<vm::LoxError>() {
            Ok(e) => match e.kind {
                vm::LoxErrorKind::RuntimeError => std::process::exit(70),
            },
            Err(_) => std::process::exit(65),
        }
    }
}
