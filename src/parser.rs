use crate::lexer::ConditionKind;
use crate::lexer::Inline;
use crate::lexer::Symbol;
use crate::lexer::Symbol::*;
use crate::lexer::Token;
use crate::lexer::TokenLine;
use crate::token_expect::match_exact_ok;
use crate::utility::GeneralExt;
use crate::utility::PackedResultExt;
use bitflags::bitflags;
use std::collections::HashSet;

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
#[derive(Debug, PartialEq, Clone, derive_new::new)]
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
#[derive(Debug, Clone)]
pub struct If {
    pub a: ArgumentCallArg,
    pub b: ArgumentCallArg,
    pub if_true: Block,
    pub if_false: Option<Block>,
    pub cond: ConditionKind,
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

#[derive(Debug, Clone, derive_new::new)]
pub struct Assignment {
    pub left: VariableId,
    pub right: ArgumentCallArg,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Inline(String),
    If(If),
    FunctionCall(FunctionCall),
    Flag(Flag),
    Ret(Ret),
    Assignment(Assignment),
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
bitflags! {
    #[derive(Debug,Default)]
    pub struct FeatureFlags: u32{
        const StackFunctions= 1<<0;
    }
}

#[derive(Default, Debug)]
pub struct ProgramTree {
    pub functions: Vec<Function>, //fnc[0]== main
    pub globals: Vec<Variable>,   //globals[1]== ret, globals[0]==temp
    pub constants_used: HashSet<i32>,
    pub features: FeatureFlags,
}

#[derive(derive_new::new)]
struct BuildingContext<'a> {
    cur_function: &'a Function,
    cur_tree: &'a ProgramTree,    //unfinished tree
    upper: Option<&'a Statement>, //kinda removed
    constants: HashSet<i32>,
}

fn is_main(f: &Function) -> bool {
    return f.name == "main";
}

impl ParserError {
    pub fn upgrade(content: String, token_line: &TokenLine) -> Self {
        return Self::new(token_line.line_number, token_line.org.clone(), content);
    }
    pub fn general(content: String) -> Self {
        return Self::new(0, "".to_owned(), content);
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
        if fnc.is_stack {
            self.features |= FeatureFlags::StackFunctions;
        }
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
    pub fn try_find_variable(&self, name: &str) -> Result<&Variable, String> {
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

///works for args NOT ending in ')'
///for args ending with ')' it will skip it at the end
fn parse_argument_next<'a>(
    tokens: &'a [Token],
    context: &BuildingContext,
) -> Result<Option<(ArgumentCallArg, &'a [Token])>, String> {
    if tokens.len() == 0 {
        return Ok(None);
    }
    if tokens.len() == 1 && tokens[0] == Token::Symbol(ParenthesisClose) {
        return Ok(None);
    }
    match &tokens[0] {
        Token::Label(label) => {
            return Ok(Some((
                ArgumentCallArg::Reference(context.try_find_variable(&label)?.id),
                &tokens[1..],
            )));
        } //TODO this
        Token::Symbol(Symbol::Asterix | Symbol::Ampersand | Symbol::Minus) => {
            //tokens.len()!=0
            if tokens.len() == 1 || !matches!(tokens[1], Token::Label(_)) {
                return Ok(None);
            }
            let label = tokens[1].to_label().unwrap(); //safe unwrap
            let symbol = Symbol::try_from(tokens[0].clone()).unwrap(); //safe unwrap
            return match symbol {
                Symbol::Asterix => Ok(Some((
                    ArgumentCallArg::Deref(context.try_find_variable(&label)?.id),
                    &tokens[2..],
                ))),
                Symbol::Ampersand => Ok(Some((
                    ArgumentCallArg::GetAddress(context.try_find_variable(&label)?.id),
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
        _ => {
            return Err(format!(
                "Incorrect token while parsing arguments {:?}",
                tokens[0]
            ))
        }
    }
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

fn parse_function_call_if_present(
    tokens: &[Token],
    context: &mut BuildingContext,
) -> Result<Option<FunctionCall>, String> {
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
        assignment = Some(context.try_find_variable(&assigment_name)?.id);
        tokens = &tokens[2..];
    }

    if !match_exact_cond(
        &[
            (TokenReq::Label, IndexReq::Next),
            (TokenReq::m_symbol(ParenthesisOpen), IndexReq::Next),
            (TokenReq::m_symbol(ParenthesisClose), IndexReq::End(0)),
        ],
        tokens,
    ) {
        return Ok(None); //not a function call
    }

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
    if (function.is_stack && (!context.cur_function.is_stack && !is_main(context.cur_function))) {
        return Err((
            format!("Can't 'call {} from {}. Stack function can be called only from other stack functions or from main",function.name,context.cur_function.name)));
    }
    return Ok(Some(FunctionCall::new(
        function.id,
        arguments_got,
        assignment,
        context.cur_function.id,
    )));
}

fn parse_inline_if_present(tokens: &[Token]) -> Result<Option<Inline>, String> {
    return match_exact_ok(
        &[
            (TokenReq::Inline, IndexReq::Next),
            (TokenReq::None, IndexReq::Next),
        ],
        tokens,
    )
    .map(|e| Inline::try_from(tokens[0].clone()).unwrap()) //this unwrap should never fail
    .pack_in_result(); //(for now) never fails, either not an inline or correct inline
}
fn parse_simple(
    tokens: &[Token],
    context: &mut BuildingContext,
) -> Result<Option<Statement>, String> {
    if tokens.len() == 0 {
        return Ok(None);
    }

    return parse_inline_if_present(tokens)
        .deep_map(|e| Statement::Inline(e.0))
        .other_else(|| {
            parse_assignment_if_present(tokens, context).deep_map(|e| Statement::Assignment(e))
        })
        .other_else(|| {
            parse_function_call_if_present(tokens, context).deep_map(|e| Statement::FunctionCall(e))
        });

    /*
    if match_exact_cond(
        &[
            (TokenReq::Inline, IndexReq::Next),
            (TokenReq::None, IndexReq::Next),
        ],
        tokens,
    ) {
        return Ok(Some(Statement::Inline(
            Inline::try_from(tokens[0].clone()).unwrap().0,
        )));
    }
    let maybe_assignment = parse_assignment_if_present(tokens, context)?;
    if let Some(assingment) = maybe_assignment {
        return Ok(Some(assingment));
    }

    if let Token::Label(label) = first {
        let call = parse_function_call(tokens, context)?;
        return Ok(Some(Statement::FunctionCall(call)));
    } else {
        return Ok(None);
    }
    */
}

fn parse_assignment_if_present(
    tokens: &[Token],
    context: &mut BuildingContext,
) -> Result<Option<Assignment>, String> {
    if !match_exact_cond(
        &[
            (TokenReq::Label, IndexReq::Next),
            (TokenReq::m_symbol(Colon), IndexReq::Next),
            (TokenReq::m_symbol(Equal), IndexReq::Next),
        ],
        tokens,
    ) {
        //not an assignment
        return Ok(None);
    }
    let left_text = tokens[0].to_label().unwrap();
    let left = context.try_find_variable(&left_text)?.id;

    let arguments = parse_call_arguments(&tokens[3..], context)?;
    if arguments.len() != 1 {
        return Err("Assignment requires exactly one argument at the right side".to_owned());
    }
    let right = arguments[0].clone();

    return Ok(Some(Assignment::new(left, right)));
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
            format!("Expected correct statement, found {:?}", line.elements),
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
    let mut tokens = tokens;
    if tokens.len() == 0 {
        return StepH::end();
    }

    let upg_0 = |t| ParserError::upgrade(t, &tokens[0]);

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

    return match name.as_str() {
        "ret" => {
            let len = tokens[0].elements.len();
            let args =
                parse_call_arguments(&tokens[0].elements[3..len - 1], context).map_err(upg_0)?;

            if args.len() != 1 {
                return Err(upg_0("Return accepts one or 0 arguments".to_owned()));
            }
            let mut id = None;
            if args.len() == 1 {
                id = Some(args[0].clone());
            }

            return StepH::deliver(Statement::Ret(Ret::new(id, context.cur_function.id)), 1);
        }
        "flag" => {
            let arg = tokens[0].elements[3]
                .to_label()
                .ok_or_else(|| upg_0("flag argument must be label".to_owned()))?;

            let has_one = tokens[0].elements.len() == 5; // [3]=label, [4]=)
            if !has_one {
                return Err(upg_0("Flag accepts exactly one arguments".to_owned()));
            }

            return StepH::deliver(Statement::Flag(Flag::new(arg)), 1);
        }
        "if" => {
            let cond_kind = tokens[0].elements[3]
                .to_cond()
                .ok_or_else(|| upg_0("If must start with condition kind".to_owned()))?;

            let arguments =
                parse_call_arguments(&&tokens[0].elements[4..], context).map_err(|e| upg_0(e))?;

            let a1 = &arguments[0];
            let a2 = &arguments[1];

            let mut i = 0;

            push_slice(&mut tokens, 1, &mut i);

            let block_if = parse_internal(&tokens, context)?.apply_a(&mut tokens, &mut i);

            match_exact_w_upgrade(
                &[
                    (TokenReq::m_symbol(Dot), IndexReq::Next),
                    (
                        TokenReq::Either(Token::m_label("else"), Token::m_label("noelse")),
                        IndexReq::Next,
                    ),
                    (TokenReq::None, IndexReq::Next),
                ],
                &tokens[0],
            )?;
            let is_else_included = tokens[0].elements[1].is_exact_label("else"); // safe indexing

            //ignore dot
            push_slice(&mut tokens, 1, &mut i);

            let mut block_else = None;
            if is_else_included {
                block_else = Some(parse_internal(&tokens, context)?.apply_a(&mut tokens, &mut i));
            }

            return StepH::deliver(
                Statement::If(If {
                    a: a1.clone(),
                    b: a2.clone(),
                    if_true: block_if,
                    if_false: block_else,
                    cond: cond_kind,
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
fn check_specific_function_exists(
    tree: &ProgramTree,
    name: &str,
    expected_args: usize,
    expected_is_stac: bool,
) -> Result<(), String> {
    let f = tree.find_fnc_with_name(name);
    if f.is_none() {
        return Err(format!("{name} function is not defined"));
    }
    let f = f.unwrap();

    if f.args != expected_args {
        let cur = f.args;
        return Err(format!(
            "{name} function definition was expected to have {expected_args} arguments but had {cur} arguments"
        ));
    }

    if f.is_stack != expected_is_stac {
        let cur = f.is_stack;
        return Err(format!(
            "{name} function sack status was expected to be {expected_is_stac} but was {cur}"
        ));
    }

    return Ok(());
}

fn check_features(tree: &ProgramTree) -> Result<(), String> {
    if tree.features.contains(FeatureFlags::StackFunctions) {
        const ERR: &'static str = "Stack feature was enabled,\nbut at least one of the required function definitions were missing or incorrect:";

        let map_er = |e| format!("{ERR}\n{e}");
        check_specific_function_exists(tree, "push", 1, false).map_err(map_er)?;
        check_specific_function_exists(tree, "pop", 0, false).map_err(map_er)?;
        check_specific_function_exists(tree, "stack_return", 1, false).map_err(map_er)?;
    }
    return Ok(());
}

pub fn parse(tokens: &[TokenLine]) -> ParserResult<ProgramTree> {
    let mut tokens = tokens;
    let mut i = 0;
    let global_variables = parse_variables(tokens)?.apply_a(&mut tokens, &mut i);

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

    let main_fnc = tree
        .find_fnc_with_name("main")
        .ok_or_else(|| ParserError::new(0, "".to_owned(), "no main function".to_owned()))?;

    if main_fnc.is_stack {
        return Err(ParserError::general(
            "main function cannot be stack function".to_owned(),
        ));
    }
    if main_fnc.args != 0 {
        return Err(ParserError::general(
            "main function cannot take arguments".to_owned(),
        ));
    }

    check_features(&tree).map_err(|e| ParserError::general(e))?;
    return Ok(tree);
}

