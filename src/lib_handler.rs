use crate::parser::ProgramTree;

pub fn merge(code: &str, lib_codes: &Vec<&str>) -> String {
    let all_lib: String = lib_codes
        .iter()
        .rev()
        .fold(String::new(), |acc, lib| merge_single(&acc, lib));

    let r = merge_single(code, &all_lib);
    return r;
}

pub fn merge_single(code: &str, lib_code: &str) -> String {
    if code.is_empty() {
        return lib_code.to_owned();
    }
    if lib_code.is_empty() {
        return code.to_owned();
    }
    let pre_code: String = code.chars().into_iter().take_while(|e| *e != '*').collect();
    let after_code = &code[pre_code.len() + 1..];
    return format!("{pre_code}{lib_code}{after_code}");
}
