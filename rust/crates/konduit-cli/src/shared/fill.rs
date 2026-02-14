/// Hydrate some types with default values where needed, sometimes deriving from already defined
/// values
pub trait Fill {
    type Error;
    fn fill(self) -> Result<Self, Self::Error>
    where
        Self: Sized;
}
