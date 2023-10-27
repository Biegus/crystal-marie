use std::collections::HashSet;

use crate::lexer::Inline;
use crate::lexer::Symbol;
use crate::lexer::Symbol::*;
use crate::lexer::Token;
use crate::lexer::TokenLine;

use crate::token_expect::match_exact;
use crate::token_expect::match_exact_cond;
use crate::token_expect::IndexReq;
use crate::token_expect::TokenReq;
use crate::utility;
use crate::utility::push_slice;
use crate::utility::LinedError;
use crate::utility::SingleStep;
use crate::utility::SingleStepH;
use crate::utility::Step;
use crate::utility::StepH;
use crate::utility::SuccessStep;

pub type ParserError = LinedError<String>;
pub type ParserResult<T> = Result<T, LinedError<String>>;

#[derive(Debug, PartialEq, Clone, Copy, Default, Hash, Eq)]
pub struct FunctionId(usize);

#[derive(derive_new::new, Debug, PartialEq, Clone, Copy, Hash, Eq, Default)]
pub struct VariableId {
    pub raw: usize,
    pub kind: VariableType,
}
#[derive(Debug, PartialEq, Clone, Copy, Hash, Eq, Default)]
pub enum VariableType {
    #[default]
    Global,
    Local(FunctionId),
}
#[derive(Debug, PartialEq, derive_new::new)]
pub struct VariableDeclaration {
    pub name: String,
    pub default_value: i32,
}
#[derive(Debug, PartialEq, Clone,derive_new::new)]
pub struct Variable {
    pub name: String,
    pub default_value: i32,
    pub id: VariableId, //not perfect that it has it. can be in wrong state (same for functions)
    pub read_only: bool,
}

pub type Block = Vec<Statement>;

#[derive(Debug, PartialEq, Clone)]
pub enum ArgumentCallArg {
    Literal(i32),
    Reference(VariableId),
    Deref(VariableId),
    GetAddress(VariableId),
    Flag(String, FunctionId), //TODO usage of string here is bad, could be solved with additional passthrough for flags
}
#[derive(Debug, PartialEq, derive_new::new, Clone)]
pub struct FunctionCall {
    pub fnc_id: FunctionId,
    pub arguments: Vec<ArgumentCallArg>,
    pub assignment: Option<VariableId>,
    pub from: FunctionId,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConditionType {
    Equal,
    More,
    Less,
}
#[derive(Debug, Clone)]
pub struct If {
    pub a: ArgumentCallArg,
    pub b: ArgumentCallArg,
    pub if_true: Block,
    pub if_false: Block,
    pub cond: ConditionType,
}

#[derive(Debug, Clone, derive_new::new)]
pub struct Flag {
    pub label: String,
}
#[derive(Debug, Clone, derive_new::new)]
pub struct Ret {
    pub ret_val: Option<ArgumentCallArg>,
    pub fnc: FunctionId,
}
#[derive(Debug, Clone)]
pub enum Statement {
    Inline(String),
    If(If),
    FunctionCall(FunctionCall),
    Flag(Flag),
    Ret(Ret),
}
#[derive(Debug, derive_new::new)]
pub struct FunctionDeclaration {
    pub name: String,
    pub arguments: Vec<String>,
    pub locals: Vec<VariableDeclaration>,
    pub is_stack: bool,
}

#[derive(Debug, Default, Clone)]
pub struct Function {
    pub id: FunctionId,
    pub name: String,
    pub content: Option<Block>, //if none then function is ghost
    pub locals: Vec<Variable>,
    pub args: usize,
    pub is_stack: bool, //use bitflags instead if more bools appear
}
#[derive(Default, Debug)]
pub struct ProgramTree {
    pub functions: Vec<Function>, //fnc[0]== main
    pub globals: Vec<Variable>,   //globals[1]== ret, globals[0]==temp
    pub constants_used: HashSet<i32>,
}

#[derive(derive_new::new)]
struct BuildingContext<'a> {
    cur_function: &'a Function,
    cur_tree: &'a ProgramTree, //unfinished tree
    upper: Option<&'a Statement>,
    constants: HashSet<i32>,
}

impl ParserError {
    pub fn upgrade(content: String, token_line: &TokenLine) -> Self {
        return Self::new(token_line.line_number, token_line.org.clone(), content);
    }
}

impl ProgramTree {
    pub fn get_temp_var(&self) -> &Variable {
        return self.get_var(VariableId {
            raw: (0),
            kind: (VariableType::Global),
        });
    }
    pub fn get_ret_var(&self) -> &Variable {
        return self.get_var(VariableId {
            raw: (1),
            kind: (VariableType::Global),
        });
    }
    pub fn find_fnc_with_name(&self, t: &str) -> Option<&Function> {
        return self.functions.iter().find(|e| e.name == t);
    }
    pub fn get_fnc(&self, t: FunctionId) -> &Function {
        return &self.functions[t.0];
    }
    pub fn get_var(&self, t: VariableId) -> &Variable {
        return match t.kind {
            VariableType::Local(func_id) => &self.get_fnc(func_id).locals[t.raw],
            VariableType::Global => &self.globals[t.raw],
        };
    }
    pub fn get_fnc_mut(&mut self, t: FunctionId) -> &mut Function {
        return &mut self.functions[t.0];
    }

    pub fn find_global_var_with_name(&self, t: &str) -> Result<&Variable, String> {
        return self
            .globals
            .iter()
            .find(|e| e.name == t)
            .ok_or_else(|| (format!("global variable {} not found", t)));
    }

    fn push_fnc<'a>(&'a mut self, decl: FunctionDeclaration) -> &'a mut Function {
        let mut fnc = Function::default();
        fnc.name = decl.name;
        fnc.args = decl.arguments.len();
        fnc.id = FunctionId(self.functions.len());
        fnc.is_stack = decl.is_stack;
        push_variables(
            decl.arguments
                .into_iter()
                .map(|e| VariableDeclaration::new(e, -21))
                .collect(),
            &mut fnc.locals,
            VariableType::Local(fnc.id),
            true,
        );
        push_variables(
            decl.locals,
            &mut fnc.locals,
            VariableType::Local(fnc.id),
            false,
        );
        self.functions.push(fnc);
        let len = self.functions.len();
        let fnc = &mut self.functions[len - 1];

        return fnc;
    }
}
impl VariableType {}

impl<'a> BuildingContext<'a> {
    pub fn try_get_variable(&self, name: &str) -> Result<&Variable, String> {
        let local = self.cur_function.locals.iter().find(|e| e.name == name);
        if let Some(value) = local {
            return Ok(value);
        }
        return self
            .cur_tree
            .find_global_var_with_name(name)
            .map_err(|e| format!("neither local nor global {} found", name));
    }
}

pub fn match_upgrade_symbol_only(t: &TokenLine, s: Symbol) -> Result<(), ParserError> {
    return match_exact_w_upgrade(
        &[
            (TokenReq::m_symbol(s), IndexReq::Next),
            (TokenReq::None, IndexReq::Next),
        ],
        t,
    );
}
pub fn match_exact_w_upgrade(
    pattern: &[(TokenReq, IndexReq)],
    token_line: &TokenLine,
) -> Result<(), ParserError> {
    return match_exact(pattern, &token_line.elements)
        .map_err(|e| ParserError::upgrade(e, token_line));
}

fn parse_argument_next<'a>(
    tokens: &'a [Token],
    context: &BuildingContext,
) -> Result<Option<(ArgumentCallArg, &'a [Token])>, String> {
    if tokens.len() == 0 {
        return Ok(None);
    }
    match &tokens[0] {
        Token::Label(label) => {
            return Ok(Some((
                ArgumentCallArg::Reference(context.try_get_variable(&label)?.id),
                &tokens[1..],
            )));
        }
        //TODO this
        Token::Symbol(Symbol::Asterix | Symbol::Ampersand | Symbol::Minus) => {
            if tokens.len() == 1 || !matches!(tokens[1], Token::Label(_)) {
                return Ok(None);
            }
            let label = tokens[1].to_label().unwrap();
            let symbol = Symbol::try_from(tokens[0].clone()).unwrap();
            return match symbol {
                Symbol::Asterix => Ok(Some((
                    ArgumentCallArg::Deref(context.try_get_variable(&label)?.id),
                    &tokens[2..],
                ))),
                Symbol::Ampersand => Ok(Some((
                    ArgumentCallArg::GetAddress(context.try_get_variable(&label)?.id),
                    &tokens[2..],
                ))),

                Symbol::Minus => Ok(Some((
                    ArgumentCallArg::Flag(label, context.cur_function.id),
                    &tokens[2..],
                ))),
                _ => panic!("logic error"),
            };
        }
        Token::Number(num) => {
            return Ok(Some((ArgumentCallArg::Literal(*num), &tokens[1..])));
        }
        _ => return Ok(None),
    };
}
//assumes tokens[0]==label
fn parse_call_arguments(
    tokens: &[Token],
    context: &mut BuildingContext,
) -> Result<Vec<ArgumentCallArg>, String> {
    let result = utility::build_step_simple(&tokens[0..], |t| parse_argument_next(t, context))?;
    for el in &result {
        if let ArgumentCallArg::Literal(v) = el {
            context.constants.insert(*v);
        }
    }
    return Ok(result);
}

fn parse_function_call(
    tokens: &[Token],
    context: &mut BuildingContext,
) -> Result<FunctionCall, String> {
    let mut tokens = tokens;
    let mut assignment = None;

    if match_exact_cond(
        &[
            (TokenReq::Label, IndexReq::Next),
            (TokenReq::m_symbol(Equal), IndexReq::Next),
        ],
        tokens,
    ) {
        let assigment_name = tokens[0].to_label().unwrap();
        assignment = Some(context.try_get_variable(&assigment_name)?.id);
        tokens = &tokens[2..];
    }

    match_exact(
        &[
            (TokenReq::Label, IndexReq::Next),
            (TokenReq::m_symbol(ParenthesisOpen), IndexReq::Next),
            (TokenReq::m_symbol(ParenthesisClose), IndexReq::End(0)),
        ],
        tokens,
    )?;

    let label = tokens[0].to_label().unwrap();

    let function = context
        .cur_tree
        .find_fnc_with_name(label.as_str())
        .ok_or_else(|| format!("function {} not found", label))?;

    let arguments_got = parse_call_arguments(&tokens[2..], context)?;

    if arguments_got.len() != function.args {
        return Err(format!(
            "Function takes {} but {} were given",
            function.args,
            arguments_got.len()
        ));
    }
    return Ok(FunctionCall::new(
        function.id,
        arguments_got,
        assignment,
        context.cur_function.id,
    ));
}

fn parse_simple(
    tokens: &[Token],
    context: &mut BuildingContext,
) -> Result<Option<Statement>, String> {
    if tokens.len() == 0 {
        return Ok(None);
    }

    let first = &tokens[0];

    //TODO should use match_exact instead
    if matches!(first, Token::Inline((_))) {
        return Ok(Some(Statement::Inline(
            Inline::try_from(tokens[0].clone()).unwrap().0,
        ))); //TODO this can crash really bad
    }

    if let Token::Label(label) = first {
        let call = parse_function_call(tokens, context)?;
        return Ok(Some(Statement::FunctionCall(call)));
    } else {
        return Ok(None);
    }
}

fn parse_internal(
    lines: &[TokenLine],
    context: &mut BuildingContext,
) -> SingleStep<Block, ParserError> {
    let mut statements: Block = Vec::new();
    let mut i = 0;

    match_upgrade_symbol_only(&lines[0], BraceOpen)?;
    i += 1;

    while i < lines.len() {
        let line = &lines[i];

        if match_exact_cond(
            &[
                (TokenReq::m_symbol(BraceClose), IndexReq::Next),
                (TokenReq::None, IndexReq::Next),
            ],
            &line.elements,
        ) {
            return SingleStepH::deliver(statements, i + 1);
        }

        let simple_maybe =
            parse_simple(&line.elements, context).map_err(|e| ParserError::upgrade(e, &line))?;

        if let Some(simple) = simple_maybe {
            statements.push(simple);
            i += 1;
            continue;
        }

        let adv_maybe = parse_advanced(&lines[i..], context)?;
        if let Some(adv) = adv_maybe {
            statements.push(adv.apply(&mut i));
            continue;
        }
        return Err(ParserError::upgrade(
            format!("Expected correct statement, found {:?}", line),
            line,
        ));
    }
    return Err(ParserError::upgrade(
        "Block was not closed".to_owned(),
        &lines[0],
    ));
}

fn parse_advanced<'a>(
    tokens: &'a [TokenLine],
    context: &mut BuildingContext,
) -> Step<Statement, ParserError> {
    //doesn't check inside
    if tokens.len() == 0 {
        return StepH::end();
    }
    let mut tokens = tokens;
    if !match_exact_cond(
        &[
            (TokenReq::m_symbol(Dot), IndexReq::Next),
            (TokenReq::Label, IndexReq::Next),
            (TokenReq::m_symbol(ParenthesisOpen), IndexReq::Next),
            (TokenReq::m_symbol(ParenthesisClose), IndexReq::End(0)),
        ],
        &tokens[0].elements,
    ) {
        return Ok(None); //wrong format
    }

    let name = String::try_from(tokens[0].elements[1].clone()).unwrap(); // checked before

    //TODO ret and flag should make sure they get exact amount of arguments
    return match name.as_str() {
        "ret" => {
            let len = tokens[0].elements.len();
            let ret_val_args = parse_call_arguments(&tokens[0].elements[3..len - 1], context)
                .map_err(|e| ParserError::upgrade(e, &tokens[0]))?;

            if ret_val_args.len() > 1 {
                return Err(ParserError::upgrade(
                    "Return accepts one or 0 arguments".to_owned(),
                    &tokens[0],
                ));
            }
            let mut id = None;
            if ret_val_args.len() == 1 {
                id = Some(ret_val_args[0].clone());
            }

            return StepH::deliver(Statement::Ret(Ret::new(id, context.cur_function.id)), 1);
        }
        "flag" => {
            let arg = tokens[0].elements[3].to_label().unwrap(); //TODO bad unwrap

            return StepH::deliver(Statement::Flag(Flag::new(arg)), 1);
        }
        "if" => {
            let cond_type_text = tokens[0].elements[3].to_label().unwrap(); //unwrap!!
            let cond_type = match cond_type_text.as_str() {
                "EQ" => Ok(ConditionType::Equal),
                "LESS" => Ok(ConditionType::Less),
                "MORE" => Ok(ConditionType::More),
                _ => {
                    return Err(ParserError::upgrade(
                        format!("unknown if type {}", cond_type_text),
                        &tokens[0],
                    ))
                }
            }?;

            let arguments = parse_call_arguments(&&tokens[0].elements[4..], context)
                .map_err(|e| ParserError::upgrade(e, &tokens[0]))?;

            let a1 = &arguments[0];
            let a2 = &arguments[1];

            let mut i = 0;

            push_slice(&mut tokens, 1, &mut i);

            let block_if = parse_internal(&tokens, context)?.apply_a(&mut tokens, &mut i);

            match_exact_w_upgrade(
                &[
                    (TokenReq::m_symbol(Dot), IndexReq::Next),
                    (TokenReq::m_label("else"), IndexReq::Next),
                    (TokenReq::None, IndexReq::Next),
                ],
                &tokens[0],
            )?;
            push_slice(&mut tokens, 1, &mut i);

            let block_else = parse_internal(&tokens, context)?.apply_a(&mut tokens, &mut i);

            return StepH::deliver(
                Statement::If(If {
                    a: a1.clone(),
                    b: a2.clone(),
                    if_true: block_if,
                    if_false: block_else,
                    cond: cond_type,
                }),
                i,
            );
        }
        _ => StepH::end(),
    };
}
pub fn parse_variable_decl(tokens: &[Token]) -> Result<VariableDeclaration, String> {
    match_exact(
        &[
            (TokenReq::Label, IndexReq::Next),
            (TokenReq::m_symbol(Equal), IndexReq::Next),
            (TokenReq::Number, IndexReq::Next),
            (TokenReq::None, IndexReq::Next),
        ],
        tokens,
    )?;

    let name = String::try_from(tokens[0].clone()).unwrap();
    let def_value = i32::try_from(tokens[2].clone()).unwrap();

    return Ok(VariableDeclaration::new(name, def_value));
}
pub fn parse_variables(
    token_lines: &[TokenLine],
) -> Result<SuccessStep<Vec<VariableDeclaration>>, ParserError> {
    let mut all = Vec::new();

    let mut i = 0;
    while i < token_lines.len() {
        let token_line = &token_lines[i];

        if match_exact_cond(
            &[
                (TokenReq::m_symbol(Symbol::Asterix), IndexReq::Beg(0)),
                (TokenReq::None, IndexReq::Beg(1)),
            ],
            &token_line.elements,
        ) {
            return Ok(SuccessStep::new(all, i + 1));
        }

        let decl = parse_variable_decl(&token_lines[i].elements)
            .map_err(|e| ParserError::upgrade(e, &token_line))?;
        all.push(decl);
        i += 1;
    }

    return Err(ParserError::upgrade(
        "Variable declaration not closed".to_owned(),
        &token_lines.last().unwrap_or(&TokenLine::none()),
    ));
}
pub fn push_variables(
    vars: Vec<VariableDeclaration>,
    dest: &mut Vec<Variable>,
    kind: VariableType,
    readonly: bool,
) {
    let mut i = dest.len();
    for var in vars {
        dest.push(Variable {
            name: var.name,
            default_value: var.default_value,
            id: VariableId::new(i, kind),
            read_only: readonly,
        });
        i += 1;
    }
}

pub fn parse_fnc_declaration<'a>(
    lines: &'a [TokenLine],
) -> SingleStep<FunctionDeclaration, ParserError> {
    let first = &lines[0];

    match_exact_w_upgrade(
        &[
            (
                TokenReq::Either(
                    Token::Label("function".to_owned()),
                    Token::Label("stack_function".to_owned()),
                ),
                IndexReq::Next,
            ),
            (TokenReq::Label, IndexReq::Next),
            (TokenReq::Label, IndexReq::Between(2, first.elements.len())),
        ],
        &first,
    )?;

    let is_stack = first.elements[0].to_label().unwrap() == "stack_function";
    let fnc_name = String::try_from(first.elements[1].clone()).unwrap();

    let args: Vec<String> = first.elements[2..]
        .iter()
        .map(|e| e.to_label().unwrap())
        .collect();

    let mut i = 1;
    let locals = parse_variables(&lines[1..])?.apply(&mut i);

    return SingleStepH::deliver(
        FunctionDeclaration::new(fnc_name, args, locals, is_stack),
        i,
    );
}

pub fn push_next_function(tokens: &[TokenLine], tree: &mut ProgramTree) -> Step<(), ParserError> {
    if tokens.len() == 0 {
        return StepH::end();
    }

    let mut i = 0;
    let decl = parse_fnc_declaration(tokens)?.apply(&mut i);

    if tree.find_fnc_with_name(&decl.name).is_some() {
        return Err(ParserError::upgrade(
            "there's a function with the same name already".to_owned(),
            &tokens[0],
        ));
    }

    let id = tree.push_fnc(decl).id;

    let mut context = BuildingContext::new(tree.get_fnc(id), &tree, None, HashSet::new());

    let block = parse_internal(&tokens[i..], &mut context)?.apply(&mut i);

    for el in context.constants {
        tree.constants_used.insert(el);
    }

    let fnc = tree.get_fnc_mut(id);
    fnc.content = Some(block);

    //TODO constant "finding" in general is terribly written

    let mut temp = HashSet::new();
    for el in &fnc.locals[fnc.args..] {
        temp.insert(el.default_value);
    }

    for el in temp {
        tree.constants_used.insert(el);
    }

    return StepH::deliver((), i);
}
pub fn parse(tokens: &[TokenLine]) -> ParserResult<ProgramTree> {
    let mut tokens = tokens;
    let mut i = 0;
    let mut global_variables = parse_variables(tokens)?.apply_a(&mut tokens, &mut i);

    let mut tree = ProgramTree::default();

    push_variables(
        vec![
            VariableDeclaration::new("_temp".to_owned(), -100), // 0
            VariableDeclaration::new("return".to_owned(), -200), //1
            VariableDeclaration::new("return_saver".to_owned(), -300), //1
        ],
        &mut tree.globals,
        VariableType::Global,
        false,
    );
    push_variables(
        global_variables,
        &mut tree.globals,
        VariableType::Global,
        false,
    );

    let mut i = 0;
    loop {
        let step = push_next_function(tokens, &mut tree)?;
        if step.is_none() {
            break;
        }
        step.unwrap().apply_a(&mut tokens, &mut i);
    }

    return Ok(tree);
}
