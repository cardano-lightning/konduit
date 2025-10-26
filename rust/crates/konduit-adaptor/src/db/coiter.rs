use std::cmp::Ordering;

pub fn coiter<K, V, I1, I2, F1, F2, F3>(
    iter1: I1,
    iter2: I2,
    mut on_left_only: F1,
    mut on_right_only: F2,
    mut on_both: F3,
) where
    K: Ord,
    I1: Iterator<Item = (K, V)>,
    I2: Iterator<Item = K>,
    F1: FnMut(K, V),
    F2: FnMut(K),
    F3: FnMut(K, V),
{
    let mut iter1 = iter1.peekable();
    let mut iter2 = iter2.peekable();
    loop {
        match (iter1.peek(), iter2.peek()) {
            (Some(_), None) => {
                let (k, v) = iter1.next().unwrap();
                on_left_only(k, v);
            }
            (None, Some(_)) => {
                let k = iter2.next().unwrap();
                on_right_only(k);
            }
            (Some((k1, _)), Some(k2)) => match k1.cmp(k2) {
                Ordering::Less => {
                    let (k, v) = iter1.next().unwrap();
                    on_left_only(k, v);
                }
                Ordering::Greater => {
                    let k = iter2.next().unwrap();
                    on_right_only(k);
                }
                Ordering::Equal => {
                    let (k, v) = iter1.next().unwrap();
                    let _ = iter2.next().unwrap();
                    on_both(k, v);
                }
            },
            (None, None) => {
                break;
            }
        }
    }
}
