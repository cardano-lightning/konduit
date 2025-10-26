use std::cmp::Ordering;
use std::iter::Peekable;

/// Folds over two iterators, giving the step function peekable access.
///
/// - `iter_a`, `iter_b`: The two iterators to consume.
/// - `acc`: The initial value of the accumulator.
/// - `step_fn`: A closure that defines the logic for each step.
///   - It receives the mutable accumulator (`&mut Acc`).
///   - It receives the two peekable iterators (`&mut Peekable<I>`).
///   - It must return `true` to continue the loop or `false` to break.
///   - Inside this closure, you are responsible for both peeking and
///     calling `.next()` to consume items.

pub fn fold_dual_peek<Acc, A, B, I, J, F>(
    iter_a: I,
    iter_b: J,
    acc: &mut Acc,
    mut stepper: F,
) -> &Acc
where
    I: Iterator<Item = A>,
    J: Iterator<Item = B>,
    F: FnMut(&mut Acc, &mut Peekable<I>, &mut Peekable<J>) -> bool,
{
    let mut peek_a = iter_a.peekable();
    let mut peek_b = iter_b.peekable();
    loop {
        if !stepper(acc, &mut peek_a, &mut peek_b) {
            break;
        }
    }
    acc
}

/// Creates a "stepper" closure for use with `fold_dual_peek`.
///
/// This stepper is designed for two iterators sorted by an `Id` key.
/// It compares the `Id`s of the next items in each iterator and calls
/// the appropriate handler.
///
/// - `on_a`: Called when an item from `iter_a` has the lowest `Id`
///           (or `iter_b` is empty).
/// - `on_b`: Called when an item from `iter_b` has the lowest `Id`
///           (or `iter_a` is empty).
/// - `on_both`: Called when items from both iterators have the same `Id`.
/// Creates a "stepper" closure for use with `fold_dual_peek`.
pub fn mk_stepper<Id, A, B, C, FA, FB, FBoth, I, J>(
    mut on_a: FA,
    mut on_b: FB,
    mut on_both: FBoth,
) -> impl FnMut(
    &mut Vec<C>,
    &mut Peekable<I>, // Use the generic type I
    &mut Peekable<J>, // Use the generic type J
) -> bool
where
    Id: Ord,
    // Add trait bounds for the new I and J parameters
    I: Iterator<Item = (Id, A)>,
    J: Iterator<Item = (Id, B)>,
    // Bounds for the handlers and data
    A: 'static,
    B: 'static,
    C: 'static,
    FA: FnMut(Id, A) -> C + 'static,
    FB: FnMut(Id, B) -> C + 'static,
    FBoth: FnMut(Id, A, B) -> C + 'static,
{
    // 'move' captures the handlers (on_a, on_b, on_both)
    move |acc, iter_a, iter_b| {
        // Peek at the IDs without consuming the items
        let id_a = iter_a.peek().map(|(id, _)| id);
        let id_b = iter_b.peek().map(|(id, _)| id);

        match (id_a, id_b) {
            // Case 1: Both iterators have items
            (Some(a_id), Some(b_id)) => {
                match a_id.cmp(b_id) {
                    Ordering::Less => {
                        // `a`'s ID is smaller, consume from `a`
                        let (id, a_val) = iter_a.next().unwrap();
                        acc.push(on_a(id, a_val));
                    }
                    Ordering::Greater => {
                        // `b`'s ID is smaller, consume from `b`
                        let (id, b_val) = iter_b.next().unwrap();
                        acc.push(on_b(id, b_val));
                    }
                    Ordering::Equal => {
                        // IDs are equal, consume from both
                        let (id, a_val) = iter_a.next().unwrap();
                        let (_, b_val) = iter_b.next().unwrap();
                        acc.push(on_both(id, a_val, b_val));
                    }
                }
            }
            // Case 2: Only `a` has items left
            (Some(_), None) => {
                let (id, a_val) = iter_a.next().unwrap();
                acc.push(on_a(id, a_val));
            }
            // Case 3: Only `b` has items left
            (None, Some(_)) => {
                let (id, b_val) = iter_b.next().unwrap();
                acc.push(on_b(id, b_val));
            }
            // Case 4: Both iterators are empty
            (None, None) => {
                return false; // Stop the loop
            }
        }
        true // Continue the loop
    }
}
