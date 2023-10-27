use core::fmt::Debug;
use core::fmt::Display;

use crate::lexer::TokenLine;

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
    pub line: usize,
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
        return Ok((SuccessStep::new(val, after)));
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

