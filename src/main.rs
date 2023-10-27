use std::{env, fs, process::exit};
mod compiler;
mod lexer;
mod parser;
mod string_builder;
mod token_expect;
mod utility;

struct Arguments {
    input_file_name: String,
    lib: Option<String>,
}

fn main() {
    let envs: Vec<String> = env::args().collect();

    let arguments = parse_arguments(&envs[1..]);
    if let None = arguments {
        println!("err while parsing arguments");
        return;
    }

    let arguments = arguments.unwrap();

    let code = fs::read_to_string(arguments.input_file_name.trim());

    if let Err(err) = code {
        println!("err while reading from file: {}", err);
        exit(1);
    }

    //push the lib at the beginning
    let mut code = code.unwrap();
    let mut line_dif = 0;
    if let Some(lib_file_name) = arguments.lib {
        let lib_code = fs::read_to_string(lib_file_name.trim());
        if let Err(err) = lib_code {
            println!("lib couldn't be loaded:{} ", err);
            exit(1);
        }
        let lib_code = lib_code.unwrap();

        line_dif += lib_code.lines().count() + 1;
        code = merge(&code, &lib_code);
    }

    let tokens = lexer::tokenize(&code);

    match parser::parse(tokens.as_slice()) {
        Ok(reprs) => {
            let code = compiler::compile(reprs);
            println!("{}", code);
        }
        Err(er) => {
            println!(
                "{}\n at line:{}",
                er.content,
                er.line as i32 + 1 - line_dif as i32
            );
        }
    }
}

fn merge(code: &str, lib_code: &str) -> String {
    let pre_code: String = code.chars().into_iter().take_while(|e| *e != '*').collect();
    let after_code = &code[pre_code.len() + 1..];
    return format!("{pre_code}{lib_code}{after_code}");
}
fn parse_arguments(args: &[String]) -> Option<Arguments> {
    let mut lib = None;
    for i in 1..args.len() {
        if args[i] == "-l" {
            lib = Some(args.get(i + 1)?.clone());
        }
    }

    return Some(Arguments {
        input_file_name: args.get(0)?.clone(),
        lib: lib,
    });
}
