/// Hydrate some types with default values where needed, sometimes deriving from already defined
/// values
pub trait Fill {
    fn fill(self, global: Self) -> Self;
}
