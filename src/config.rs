use crate::argument_parser::{self, ArgumentMapping};

pub enum OutputMethod {
    File(String),
    Stdout,
}

pub struct Config {
    pub input_file_name: Option<String>,
    pub output: OutputMethod,
    pub lib: Vec<String>,
    pub help_mode: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            input_file_name: None,
            output: OutputMethod::File("a.marie".to_owned()),
            lib: Vec::new(),
            help_mode: false,
        }
    }
}

pub fn parse_arguments(args: &[String]) -> Result<Config, String> {
    let parse_arguments = argument_parser::parse_arguments::<Config>(
        args,
        || Config::default(),
        |e, c| {
            c.input_file_name = e.map(|e| e.to_owned());
            return Ok(());
        },
        vec![
            ArgumentMapping::new('l', |e: &[String], c: &mut Config| {
                if e.len() < 1 {
                    return Err("after -l, specify at least one lib file".to_owned());
                }
                c.lib = e.iter().map(|e| e.clone()).collect();
                return Ok(());
            }),
            ArgumentMapping::new('o', |e: &[String], c: &mut Config| {
                if e.len() != 1 {
                    return Err("o argument takes exactly one value".to_owned());
                }
                c.output = OutputMethod::File(e[0].clone());
                return Ok(());
            }),
            ArgumentMapping::new('s', |e: &[String], c: &mut Config| {
                if e.len() != 0 {
                    return Err("s argument takes no values ".to_owned());
                }
                c.output = OutputMethod::Stdout;
                return Ok(());
            }),
        ],
    );
    return parse_arguments;
}
