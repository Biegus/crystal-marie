use std::fmt::Display;


pub fn push(builder: &mut Vec<char>, v: &str) {
    builder.extend_from_slice(v.chars().collect::<Vec<char>>().as_slice());
}
pub fn push_line(builder: &mut Vec<char>, v: &str) {
    push(builder, format!("{}\n", v).as_str());
}
pub fn fold_additive<'a, I, T, F>(slice: I, operation: F) -> String
where
    I: Iterator<Item = T>,
    F: Fn(T) -> String,
{
    let mut t: Vec<char> = Vec::new();
    for var in slice {
        push(&mut t, operation(var).as_str());
    }
    return collapse(t);
}
pub fn collapse(v: impl IntoIterator<Item = char>) -> String {
    return v.into_iter().collect();
}
pub fn bulk<T, It>(it: T) -> String
where
    T: Iterator<Item = It>,
    It: Display,
{
    return fold_additive(it, |a| a.to_string());
}

pub struct Builder(pub Vec<char>, usize);
impl Builder {
    pub fn new() -> Builder {
        return Builder(vec![], 0);
    }
    pub fn count(&self) -> usize {
        return self.1;
    }
    pub fn push(&mut self, t: &str) {
        self.1 += t.chars().into_iter().filter(|e| *e == '\n').count();
        push(&mut self.0, t);
    }
    pub fn push_line_smart(&mut self, t: &str) -> &mut Builder {
        if t == "" {
            return self;
        }
        self.1 += t.chars().into_iter().filter(|e| *e == '\n').count() + 1;
        push_line(&mut self.0, t);
        return self;
    }
    pub fn collapse(self) -> String {
        return collapse(self.0);
    }
    //TODO
    pub fn collapse_flat(self) -> String {
        let mut m_self = self;
        while m_self.0.len() > 0 && m_self.0[m_self.0.len() - 1] == '\n' {
            m_self.0.pop();
        }
        return collapse(m_self.0);
    }
}
