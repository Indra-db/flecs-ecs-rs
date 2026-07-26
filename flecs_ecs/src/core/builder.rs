#![doc(hidden)]

pub trait Builder<'a>: Sized {
    type BuiltType;

    fn build(self) -> Self::BuiltType;
}
