use crate::{
    lexer::ConditionKind,
    parser::{
        ArgumentCallArg, Function, FunctionCall, FunctionId, If, ProgramTree, Statement, Variable,
        VariableId, VariableType,
    },
    string_builder::Builder,
};

use std::collections::{HashMap, HashSet};
#[derive(Default)]
pub struct CompilerContext {
    address_map: HashMap<VariableId, usize>,
    tree: ProgramTree,
    element_counter: usize,
    flags: Vec<(String, usize)>,
    addrs: Vec<(usize, usize)>,
}
impl CompilerContext {
    fn push_counter(&mut self) -> usize {
        self.element_counter += 1;
        return self.element_counter - 1;
    }
}
pub fn var_decl_text(t: &Variable, tree: &ProgramTree) -> String {
    return format!("{}, DEC {}", get_var_text(t, tree), t.default_value);
}
pub fn constant_decl_text(t: i32) -> String {
    return format!("const_{t}, DEC {t}");
}
pub fn get_var_text(t: &Variable, tree: &ProgramTree) -> String {
    return match t.id.kind {
        VariableType::Global => format!("var_{}", t.name),
        VariableType::Local(id) => format!("var_{}_{}", tree.get_fnc(id).name, t.name),
    };
}
pub fn get_constant_text(n: i32) -> String {
    return format!("const_{n}");
}

fn get_load_from_arg_text(arg: &ArgumentCallArg, context: &CompilerContext) -> String {
    return match arg {
        ArgumentCallArg::Literal(v) => {
            format!("load {}", get_constant_text(*v))
        }
        ArgumentCallArg::Reference(value_id) => {
            format!(
                "load {}",
                get_var_text(context.tree.get_var(*value_id), &context.tree)
            )
        }
        ArgumentCallArg::Deref(value_id) => {
            format!(
                "loadi {}",
                get_var_text(&context.tree.get_var(*value_id), &context.tree)
            )
        }
        ArgumentCallArg::GetAddress(id) => {
            format!("load {}", get_constant_text(context.address_map[id] as i32))
        }
        ArgumentCallArg::Flag(content, id) => {
            format!("load flag_{}_{}", context.tree.get_fnc(*id).name, content)
        }
    };
}

fn get_store_text(arg: VariableId, context: &CompilerContext) -> String {
    return format!(
        "store {}",
        get_var_text(context.tree.get_var(arg), &context.tree)
    );
}
fn get_set_from_arg(var: &Variable, arg: &ArgumentCallArg, context: &CompilerContext) -> String {
    let var_t = get_var_text(var, &context.tree);
    return format!("{}\nstore {var_t}", get_load_from_arg_text(arg, context));
}

fn get_set_var_to_num_text(var: &Variable, value: i32, tree: &ProgramTree) -> String {
    let var_t = get_var_text(var, tree);
    let value_t = get_constant_text(value);
    return format!("load {value_t}\nstore {var_t}");
}

fn get_set_var_to_other_text(var: &Variable, value: &Variable, tree: &ProgramTree) -> String {
    let var_t = get_var_text(var, tree);
    let value_t = get_var_text(value, tree);
    return format!("load {value_t}\nstore {var_t}");
}

pub fn compile_variables(context: &mut CompilerContext, count: usize) -> String {
    let mut builder = Builder::new();
    //handling constants
    //todo the whole handling of constants here is prety dirty
    let mut all_constants = HashSet::new();
    all_constants.extend(context.tree.constants_used.clone());
    let v: Vec<i32> = context.address_map.iter().map(|e| *e.1 as i32).collect();
    all_constants.extend(v);
    let mut constants_vector: Vec<_> = all_constants.iter().collect();
    constants_vector.sort();
    for el in &constants_vector {
        builder.push_line_smart(&constant_decl_text(**el));
    }

    //global variables
    for el in &context.tree.globals {
        builder.push_line_smart(&var_decl_text(&el, &context.tree));
        context
            .address_map
            .insert(el.id, builder.count() + count - 1);
    }

    //local variables
    for fnc in &context.tree.functions {
        for el in &fnc.locals {
            builder.push_line_smart(&var_decl_text(&el, &context.tree));
            context
                .address_map
                .insert(el.id, builder.count() + count - 1);
        }
    }

    return builder.collapse_flat();
}

pub fn get_real_fnc_name_text(t: &Function) -> String {
    return format!("function_{}", t.name);
}
pub fn compile_simple(
    st: &Statement,
    context: &mut CompilerContext,
    lines: usize,
) -> Option<String> {
    match st {
        Statement::Ret(ret) => {
            let fnc = context.tree.get_fnc(ret.fnc);
            return Some(get_function_return_text(fnc, ret.ret_val.as_ref(), context));
        }
        Statement::Inline(inline) => Some(inline.clone().trim().into()),
        Statement::FunctionCall(call) => {
            let fnc = context.tree.get_fnc(call.fnc_id);
            match fnc.is_stack {
                false => return Some(get_normal_function_call_text(call, context)),
                true => return Some(get_stack_function_call_text(call, context, lines)),
            }
        }
        Statement::Assignment(ass) => {
            return Some(get_set_from_arg(
                context.tree.get_var(ass.left),
                &ass.right,
                &context,
            ));
        }
        _ => None,
    }
}

fn get_stack_function_call_text(
    master_call: &FunctionCall,
    context: &mut CompilerContext,
    lines: usize,
) -> String {
    //TODO this function is horrible many hardcodes

    let counter = context.push_counter();

    let push_func = context.tree.find_fnc_with_name("push").unwrap(); //TODO this may crash , theres no error propagation in this part
    let arg_name = format!("var_push_{}", push_func.locals[0].name);
    let push_func = push_func.id;

    let pop_func = context.tree.find_fnc_with_name("pop").unwrap().id; //TODO this may crash
    let mut builder = Builder::new();

    //normal function CANT call stack functions except for main
    // we should save arguments only if we are ourself a stack-function
    // from_stack is actually the same as (is not main)

    let from = context.tree.get_fnc(master_call.from);
    let from_id = from.id;
    let from_stack = from.is_stack;
    if from_stack {
        //save locals fom being lost

        for arg in &from.locals {
            let push_call = FunctionCall::new(
                push_func,
                vec![ArgumentCallArg::Reference(arg.id)],
                None,
                from_id,
            );
            builder.push_line_smart(&get_normal_function_call_text(&push_call, context));
        }
    }

    // save the address (terribly tho)
    let addr = format!("addr_{}", counter);

    builder.push_line_smart(format!("load {addr}").as_str());
    builder.push_line_smart(format!("store {arg_name}").as_str());
    builder.push_line_smart(format!("jns function_push").as_str());

    //push call arguments
    for el in &master_call.arguments {
        let push_call = FunctionCall::new(push_func, vec![el.clone()], None, from_id);
        builder.push_line_smart(&get_normal_function_call_text(&push_call, context));
    }

    //do the call
    builder.push_line_smart(&format!(
        "jns {}",
        get_real_fnc_name_text(context.tree.get_fnc(master_call.fnc_id))
    ));
    context.addrs.push((counter, builder.count() + lines));

    //while restoring locals return value would be lost
    builder.push_line_smart("load var_return\nstore var_return_saver");

    if from_stack {
        //restore locals
        for arg in from.locals.iter().rev() {
            //reversed since stuck and stuff
            let push_call = FunctionCall::new(pop_func, vec![], Some(arg.id), from_id);
            builder.push_line_smart(&get_normal_function_call_text(&push_call, context));
        }
    }
    builder.push_line_smart("load var_return_saver\nstore var_return");

    if let Some(assignment) = master_call.assignment {
        builder.push_line_smart(
            format!(
                "store {}",
                get_var_text(context.tree.get_var(assignment), &context.tree)
            )
            .as_str(),
        );
    }
    return builder.collapse_flat();
}

pub fn get_normal_function_call_text(call: &FunctionCall, context: &CompilerContext) -> String {
    let tree = &context.tree;
    let mut builder = Builder::new();
    let fnc = tree.get_fnc(call.fnc_id);
    let mut i = 0;
    for arg in &call.arguments {
        builder.push_line_smart(&get_set_from_arg(&fnc.locals[i], arg, context));
        i += 1;
    }
    builder.push_line_smart(&format!("jns {}", get_real_fnc_name_text(fnc)));
    if let Some(assignment) = call.assignment {
        builder.push_line_smart(&get_store_text(assignment, context));
    }
    let t = builder.collapse_flat();
    return t;
}

pub fn compile_advance(
    st: &Statement,
    context: &mut CompilerContext,
    line: usize,
    id: FunctionId,
) -> Option<String> {
    match st {
        Statement::Flag(flag) => {
            let fnc = context.tree.get_fnc(id);
            context
                .flags
                .push((format!("{}_{}", fnc.name, flag.label), line));
            return Some("".to_owned());
        }
        Statement::If(If {
            a,
            b,
            if_true,
            if_false: else_block,
            cond,
        }) => {
            //TODO THIS IS HELL
            let mut builder = Builder::new();
            let cond_number = match cond {
                ConditionKind::Eq => 400,
                ConditionKind::Less => 000,
                ConditionKind::More => 800,
            };
            let contains_else = else_block.is_some();
            let counter = context.push_counter();

            let jump_end_if = format!("jump end_if_{}", counter);
            let jump_else = format!("jump else_{counter}");

            let jump_if_not = if contains_else {
                jump_else.as_ref()
            } else {
                jump_end_if.as_ref()
            };

            let jump_if_ok = format!("jump if_{}", counter);

            // acc = a-b
            builder.push_line_smart(&get_set_from_arg(context.tree.get_temp_var(), b, &context));
            builder.push_line_smart(&get_load_from_arg_text(a, &context));
            builder.push_line_smart("subt var__temp");

            builder.push_line_smart(&("skipcond ".to_owned() + cond_number.to_string().as_str()));

            //if else is not present will jump to end_if
            builder.push_line_smart(&jump_if_not);
            builder.push_line_smart(&jump_if_ok);

            builder.push_line_smart(&format!("if_{},store var__temp", counter));
            builder.push_line_smart(&compile_block(if_true, context, line + builder.count(), id));

            if contains_else {
                //we only need that if "else" is present, otherwise "endif" block is right after the end of "if" bloc
                builder.push_line_smart(jump_end_if.as_ref());

                builder.push_line_smart(&format!("else_{},store var__temp", counter));
                builder.push_line_smart(&compile_block(
                    else_block.as_ref().unwrap(),
                    context,
                    line + builder.count(),
                    id,
                ));
            }

            builder.push_line_smart(&format!("end_if_{},store var__temp", counter));
            return Some(builder.collapse_flat());
        }
        _ => None,
    }
}

pub fn compile_function(fnc: FunctionId, context: &mut CompilerContext, line: usize) -> String {
    let fnc = context.tree.get_fnc(fnc);
    if fnc.content.is_none() {
        return "".to_owned(); //this funciton is a ghotst
    }
    let content = fnc.content.as_ref().unwrap();
    let mut builder = Builder::new();

    builder.push_line_smart(&format!("function_{},DEC 0", fnc.name));

    // reset locals
    for var in &fnc.locals[fnc.args..] {
        builder.push_line_smart(&get_set_var_to_num_text(
            var,
            var.default_value,
            &context.tree,
        ));
    }
    let is_stack = fnc.is_stack;
    if is_stack {
        //we need to pop arguments
        let pop_fnc = context.tree.find_fnc_with_name("pop").unwrap(); //todo may crash  no error propagation here

        //popping so reverse
        for arg in fnc.locals[0..fnc.args].iter().rev() {
            //we only pop arguments, other locals are fresh
            let pop_call = FunctionCall::new(pop_fnc.id, vec![], Some(arg.id), fnc.id);
            builder.push_line_smart(&get_normal_function_call_text(&pop_call, context));
        }
    }

    let id = fnc.id;
    if content.len() > 0 {
        builder.push_line_smart(&compile_block(
            &content.clone(),
            context,
            builder.count() + line,
            id,
        ));
    }

    if is_stack {
        let pop = context.tree.find_fnc_with_name("pop").unwrap(); //todo getting pop again...
        let pop_call = FunctionCall::new(pop.id, vec![], None, id);
        builder.push_line_smart(&get_normal_function_call_text(&pop_call, context));
        builder.push_line_smart(format!("jumpi var_return").as_str());
    } else {
        //notstac
        builder.push_line_smart(&get_function_return_text(
            context.tree.get_fnc(id),
            None,
            context,
        )); //TODO forgetting and getting function again bad
    }
    return builder.collapse_flat();
}
pub fn get_function_return_text(
    fnc: &Function,
    ret_val: Option<&ArgumentCallArg>,
    context: &CompilerContext,
) -> String {
    if !fnc.is_stack {
        return format!(
            "{load_op}{store_op}\njumpI {fnc_name}",
            load_op = if let Some(val) = ret_val {
                get_load_from_arg_text(&val, context) + "\n"
            } else {
                "".to_owned()
            },
            store_op = get_store_text(context.tree.get_ret_var().id, context),
            fnc_name = get_real_fnc_name_text(fnc)
        );
    } else {
        let stack_fnc = context.tree.find_fnc_with_name("stack_return").unwrap();
        let args = if let Some(val) = ret_val {
            vec![val.clone()]
        } else {
            vec![]
        };
        let stack_return_call = FunctionCall::new(stack_fnc.id, args, None, fnc.id);
        return get_normal_function_call_text(&stack_return_call, context);
    }
}

pub fn compile_block(
    block: &Vec<Statement>,
    context: &mut CompilerContext,
    lines: usize,
    id: FunctionId,
) -> String {
    let tree = &context.tree;
    //do the code
    let mut builder = Builder::new();
    for statement in block {
        let simple_maybe = compile_simple(&statement, context, builder.count() + lines);

        if let Some(simple) = simple_maybe {
            builder.push_line_smart(&simple);
            continue;
        }
        let adv_maybe = compile_advance(&statement, context, builder.count() + lines, id);
        if let Some(adv) = adv_maybe {
            if adv != "" {
                builder.push_line_smart(&adv);
            }
        }
    }
    let t = builder.collapse_flat();
    return t;
}

fn compile_lower_kind_variables(context: &CompilerContext) -> String {
    let mut builder = Builder::new();
    for el in &context.flags {
        builder.push_line_smart(format!("flag_{}, dec {}", el.0, el.1).as_str());
    }
    for el in &context.addrs {
        let counter = el.0;
        let val = el.1;
        builder.push_line_smart(format!("addr_{}, dec {}", counter, val).as_str());
    }
    return builder.collapse_flat();
}

///
/// Last compilation step, compiles programtree to .marie code
///
///the tree outputed from "parser" as OK is always assumed to be program that's correct.
///Thats why this function never outpus an error.
///If this funciton panic it means either the tree was not from parser or parser has errors in its code
pub fn compile(tree: ProgramTree) -> String {
    let mut builder = Builder::new();
    let mut context = CompilerContext::default();
    context.tree = tree;

    builder.push_line_smart("jns function_main");
    builder.push_line_smart("halt");

    builder.push_line_smart(&compile_variables(&mut context, builder.count()));
    let ids: Vec<_> = context.tree.functions.iter().map(|e| e.id).collect();
    for fnc in ids {
        builder.push_line_smart(&compile_function(fnc, &mut context, builder.count()));
    }
    builder.push_line_smart(&compile_lower_kind_variables(&context));
    return builder.collapse_flat();
}
