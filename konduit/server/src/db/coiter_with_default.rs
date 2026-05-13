use std::cmp::Ordering;

pub fn coiter_with_default<K, V, I1, I2, F>(iter1: I1, iter2: I2, mut update_fn: F)
where
    K: Ord,
    V: Default,
    I1: Iterator<Item = (K, V)>,
    I2: Iterator<Item = K>,
    F: FnMut(K, V),
{
    let mut iter1 = iter1.peekable();
    let mut iter2 = iter2.peekable();

    loop {
        match (iter1.peek(), iter2.peek()) {
            (Some(_), None) => {
                let (k, v) = iter1.next().unwrap();
                update_fn(k, v);
            }
            (None, Some(_)) => {
                let k = iter2.next().unwrap();
                update_fn(k, V::default());
            }
            (Some((k1, _)), Some(k2)) => match k1.cmp(k2) {
                Ordering::Less => {
                    let (k, v) = iter1.next().unwrap();
                    update_fn(k, v);
                }
                Ordering::Greater => {
                    let k = iter2.next().unwrap();
                    update_fn(k, V::default());
                }
                Ordering::Equal => {
                    let (k, v) = iter1.next().unwrap();
                    let _ = iter2.next().unwrap();
                    update_fn(k, v);
                }
            },
            (None, None) => {
                break;
            }
        }
    }
}
