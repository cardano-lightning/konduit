use anyhow::{Result, anyhow};

pub fn v2a<T, const N: usize>(v: Vec<T>) -> Result<[T; N]> {
    <[T; N]>::try_from(v)
        .map_err(|v: Vec<T>| anyhow!("Expected a Vec of length {}, but got {}", N, v.len()))
}

pub fn concat<T: Clone>(l: &[T], r: &[T]) -> Vec<T> {
    let mut n = l.to_vec();
    n.extend(r.iter().cloned());
    return n;
}

pub fn unzip<A, B>(zipped: Vec<(A, B)>) -> (Vec<A>, Vec<B>) {
    let mut va: Vec<A> = Vec::with_capacity(zipped.len());
    let mut vb: Vec<B> = Vec::with_capacity(zipped.len());
    for (a, b) in zipped.into_iter() {
        va.push(a);
        vb.push(b);
    }
    (va, vb)
}
