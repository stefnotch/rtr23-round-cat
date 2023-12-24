use std::ops::Deref;

/// A borrowed or owned value.
/// Like a `Cow`, but without the `Clone` requirement.
pub enum Bow<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<'a, T> Bow<'a, T> {
    pub fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<'a, T> From<&'a T> for Bow<'a, T> {
    fn from(value: &'a T) -> Self {
        Self::Borrowed(value)
    }
}

impl<T> From<T> for Bow<'_, T> {
    fn from(value: T) -> Self {
        Self::Owned(value)
    }
}

impl<T> Deref for Bow<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(value) => value,
            Self::Owned(value) => value,
        }
    }
}
