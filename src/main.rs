use core::fmt;
use std::{collections::HashSet, convert::Infallible, env, fs, path::Display, process::exit};

use argument_parser::ArgumentMapping;
use config::OutputMethod;
use lib_handler::merge_single;

mod argument_parser;
mod compiler;
mod config;
mod lexer;
mod lib_handler;
mod parser;
mod string_builder;
mod token_expect;
mod utility;
fn print_help() {}
fn main() {
    let envs: Vec<String> = env::args().collect();
    let arguments = config::parse_arguments(&envs[1..]);
    if let Err(err) = arguments {
        eprintln!("err while parsing arguments: {}", err);
        exit(1);
    }
    let arguments = arguments.unwrap();

    if arguments.help_mode {
        print_help();
        exit(0);
    }

    if arguments.input_file_name.is_none() {
        eprintln!("err: input file name not specified");
        exit(1);
    }
    let input_file_name = arguments.input_file_name.unwrap();
    let code = fs::read_to_string(input_file_name.trim());

    if let Err(err) = code {
        eprintln!("err while reading from file: {}", err);
        exit(1);
    }

    //push the lib at the beginning
    let mut main_code = code.unwrap();
    let lib_code: Result<Vec<_>, _> = arguments
        .lib
        .iter()
        .map(|e| (fs::read_to_string(e.trim()).map(|content| (content, e.clone()))))
        .collect();

    if let Err(error) = lib_code {
        eprintln!("lib couldn't be loaded:{} ", error);
        exit(1);
    }
    let lib_code = lib_code.unwrap();

    let mut line_dif = 1; // its one cause its hacky way to avoid it getting in the way if not used

    if !arguments.lib.is_empty() {
        let lib_code: Result<Vec<_>, _> = arguments
            .lib
            .iter()
            .map(|e| fs::read_to_string(e.trim()))
            .collect();
        if let Err(err) = lib_code {
            eprintln!("lib couldn't be loaded:{} ", err);
            exit(1);
        }
        let lib_code = lib_code.unwrap();

        line_dif = lib_code
            .iter()
            .map(|e| e.lines().count() - 1)
            .sum::<usize>()
            + 2;

        main_code = lib_handler::merge(
            &main_code,
            &lib_code.iter().map(|e| e.as_str()).collect::<Vec<&str>>(),
        );
    }

    let tokens = lexer::tokenize(&main_code);

    let maybe_parsed = parser::parse(tokens.as_slice());

    if let Err(err) = maybe_parsed {
        eprintln!(
            "{}\n\nat line:{}\n\"{}\"\n\nTokens at current line:\n\"{:?}\"",
            err.content,
            err.line as i32 + 1 - (line_dif - 1) as i32,
            //           line_to_file.iter().find(|e| )
            main_code
                .lines()
                .nth(err.line)
                .unwrap_or("Error while trying to get line giving error"), // it can go negative in some cases (kinda bug)
            tokens
                .iter()
                .filter(|e| e.line_number == err.line)
                .nth(0)
                .unwrap()
                .elements
        );
        exit(1)
    }
    let reprs = maybe_parsed.unwrap();
    let code = compiler::compile(reprs);

    match arguments.output {
        OutputMethod::File(file_name) => {
            let write_res = fs::write(file_name, code);
            if let Err(err) = write_res {
                eprintln!("err while writing to file: {}", err);
                exit(1);
            }
        }
        OutputMethod::Stdout => println!("{}", code),
    }
}

//todo: lexer may crash (if given very big number)
