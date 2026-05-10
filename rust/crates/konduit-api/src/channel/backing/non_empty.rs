// ---------------------------------------------------------------------------
// NonEmpty<T> — a Vec guaranteed to have at least one element
// ---------------------------------------------------------------------------

use minicbor::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize, Encode, Decode,
)]
pub struct NonEmpty<T> {
    #[n(0)]
    head: T,
    #[n(1)]
    tail: Vec<T>,
}

impl<T> NonEmpty<T> {
    pub fn singleton(value: T) -> Self {
        Self {
            head: value,
            tail: vec![],
        }
    }

    pub fn push(&mut self, value: T) {
        self.tail.push(value);
    }

    pub fn head(&self) -> &T {
        &self.head
    }

    pub fn tail(&self) -> &[T] {
        &self.tail
    }

    pub fn tail_mut(&mut self) -> &mut Vec<T> {
        &mut self.tail
    }

    pub fn last(&self) -> &T {
        self.tail.last().unwrap_or(&self.head)
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.head).chain(self.tail.iter())
    }

    pub fn len(&self) -> usize {
        1 + self.tail.len()
    }
}

impl<T> TryFrom<Vec<T>> for NonEmpty<T> {
    type Error = ();

    fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
        match value.as_slice() {
            [] => Err(()),
            [_, ..] => {
                let mut tail = value;
                let head = tail.remove(0);
                Ok(Self { head, tail })
            }
        }
    }
}
