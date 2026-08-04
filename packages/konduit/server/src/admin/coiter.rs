use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::btree_map::IntoIter as BTreeIntoIter;
use std::iter::Peekable;
use std::vec::IntoIter;

pub enum CoItem<K, V> {
    Left(K),
    Right(K, V),
    Both(K, V),
}

pub struct CoIter<K, V> {
    left: Peekable<IntoIter<K>>,
    right: Peekable<BTreeIntoIter<K, V>>,
}

impl<K: Ord, V> CoIter<K, V> {
    pub fn new(left: Vec<K>, right: BTreeMap<K, V>) -> Self {
        Self {
            left: left.into_iter().peekable(),
            right: right.into_iter().peekable(),
        }
    }
}

impl<K: Ord, V> Iterator for CoIter<K, V> {
    type Item = CoItem<K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        match (self.left.peek(), self.right.peek()) {
            (Some(l), Some((r, _))) => match l.cmp(r) {
                Ordering::Less => self.left.next().map(CoItem::Left),
                Ordering::Greater => self.right.next().map(|(k, v)| CoItem::Right(k, v)),
                Ordering::Equal => {
                    let l = self.left.next().unwrap();
                    let (_, v) = self.right.next().unwrap();
                    Some(CoItem::Both(l, v))
                }
            },
            (Some(_), None) => self.left.next().map(CoItem::Left),
            (None, Some(_)) => self.right.next().map(|(k, v)| CoItem::Right(k, v)),
            (None, None) => None,
        }
    }
}
