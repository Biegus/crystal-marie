use std::fmt::{Debug, Display};

pub enum AString<'a> {
    String(String),
    Str(&'a str),
}
impl<'a> AString<'a> {
    pub fn as_ref(&self) -> &str {
        return match self {
            AString::String(t) => &t,
            AString::Str(t) => t,
        };
    }
}

pub trait StringExtension<'a> {
    fn a_string(self) -> AString<'a>;
}

impl StringExtension<'static> for String {
    fn a_string(self) -> AString<'static> {
        return AString::String(self);
    }
}
impl<'a> StringExtension<'a> for &'a str {
    fn a_string(self) -> AString<'a> {
        return AString::Str(self);
    }
}

pub fn build_step_simple<T, TN, F, TR>(a: &[T], f: F) -> Result<Vec<TN>, TR>
where
    F: Fn(&[T]) -> Result<Option<(TN, &[T])>, TR>,
{
    let mut a = a;
    let mut vec = Vec::new();
    loop {
        let result = f(a)?;
        match result {
            None => break,
            Some(v) => {
                a = v.1;
                vec.push(v.0);
            }
        }
    }
    return Ok(vec);
}

pub fn build_step<T, F, TR>(a: &[T], f: F) -> Result<Vec<T>, TR>
where
    F: Fn(&[T]) -> Step<T, TR>,
{
    let mut a = a;
    let mut vec = Vec::new();
    loop {
        let result = f(a)?;
        match result {
            None => break,
            Some(v) => {
                vec.push(v.val);
                a = &a[v.change..];
            }
        }
    }
    return Ok(vec);
}

#[derive(Debug, derive_new::new)]
pub struct LinedError<T>
where
    T: Debug + Display,
{
    pub line: usize, // starts from 0
    pub related_text: String,
    pub content: T,
}

#[derive(Debug, derive_new::new)]
pub struct SuccessStep<T> {
    val: T,
    change: usize,
}
pub type SingleStep<T, ERR> = Result<SuccessStep<T>, ERR>;
pub type Step<T, E> = Result<Option<SuccessStep<T>>, E>;
pub struct StepH;

impl StepH {
    pub fn end<T, E>() -> Step<T, E> {
        return Ok(None);
    }

    pub fn deliver<T, E>(val: T, after: usize) -> Step<T, E> {
        return Ok(Some(SuccessStep::new(val, after)));
    }
}

pub struct SingleStepH;

impl SingleStepH {
    pub fn deliver<T, E>(val: T, after: usize) -> SingleStep<T, E> {
        return Ok(SuccessStep::new(val, after));
    }
}

pub trait StepExtension<T, E> {
    fn end_as_err<F>(self, f: F) -> Result<SuccessStep<T>, E>
    where
        F: FnOnce() -> E;
}
impl<T, E> StepExtension<T, E> for Step<T, E> {
    fn end_as_err<F>(self, f: F) -> Result<SuccessStep<T>, E>
    where
        F: FnOnce() -> E,
    {
        return self?.ok_or_else(|| f());
    }
}

impl<T> SuccessStep<T> {
    pub fn apply(self, i: &mut usize) -> T {
        *i += self.change;
        return self.val;
    }
    pub fn apply_a<A>(self, t: &mut &[A], counter: &mut usize) -> T {
        push_slice(t, self.change, counter);
        return self.val;
    }
}
pub fn push_slice<T>(t: &mut &[T], change: usize, counter: &mut usize) {
    *t = &(*t)[change..];
    *counter += change;
}

/*
pub trait ResultOptionExtension<T> {
    fn other(self, val: Self) -> Self;
    fn other_else<F>(self, f: F) -> Self
    where
        F: Fn(Result<Option<T>, String>) -> Result<Option<T>, String>;
}

impl<T> ResultOptionExtension<Result<Option<T>, String>> for Result<Option<T>, String> {
    fn other(self, val: ) -> Self {
        let t = self.map(|e| e.or(val));
        return t;
    }

    fn other_else<F>(self, f: F) -> Self
    where
        F: Fn(Self) -> Self,
    {
        self.map(|e| e.or_else(f))
    }
}
*/
#[easy_ext::ext(GeneralExt)]
pub impl<T, E> T
where
    T: Sized,
{
    fn pack_in_result(self) -> Result<T, E> {
        return Ok(self);
    }
}
#[easy_ext::ext(ResultExt)]
pub impl<T, E> Result<T, E> {
    fn pack_option_inside(self) -> Result<Option<T>, E> {
        return self.map(|e| Some(e));
    }
}

#[easy_ext::ext(PackedResultExt)]
pub impl<T, E> Result<Option<T>, E> {
    fn other(self, v: Result<Option<T>, E>) -> Result<Option<T>, E> {
        return self.and_then(|e| match e {
            Some(org) => Ok(Some(org)),
            None => v,
        });
    }

    fn other_else<F>(self, f: F) -> Result<Option<T>, E>
    where
        F: FnOnce() -> Result<Option<T>, E>,
    {
        return self.and_then(|e| match e {
            Some(org) => Ok(Some(org)),
            None => f(),
        });
    }
    fn deep_map<F, T2>(self, f: F) -> Result<Option<T2>, E>
    where
        F: FnOnce(T) -> T2,
    {
        return self.map(|e| e.map(f));
    }
}
