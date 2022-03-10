macro_rules! error_loop {
    ($errors:expr, Result<$success:ty, $fail:ty>; $inner:expr) => {{
        let closure = || $inner;
        let res: Result<$success, $fail> = closure();
        match res {
            Ok(val) => val,
            Err(e) => {
                $errors.push(e);
                continue;
            }
        }
    }};
}

pub fn combine_syn_errors(errors: Vec<syn::Error>) -> Result<(), syn::Error> {
    let mut iter = errors.into_iter();
    if let Some(mut first) = iter.next() {
        for value in iter {
            first.combine(value);
        }
        Err(first)
    } else {
        Ok(())
    }
}

pub fn collect_vec_combining_errors<T, E>(
    iter: impl Iterator<Item = Result<T, E>>,
    error_combiner: impl FnOnce(Vec<E>) -> Result<(), E>,
) -> Result<Vec<T>, E> {
    let mut success = Vec::with_capacity(iter.size_hint().0);
    let mut errors = Vec::new();
    for value in iter {
        match value {
            Ok(succ) => success.push(succ),
            Err(e) => errors.push(e),
        }
    }
    error_combiner(errors)?;
    Ok(success)
}
