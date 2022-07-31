use crate::errors::Errors;

pub struct Valid<T>(pub T);

pub trait TryIntoValid<T> {
    fn try_into_valid(self) -> Result<Valid<T>, Errors>;
}

impl<T: validator::Validate> TryIntoValid<T> for T {
    fn try_into_valid(self) -> Result<Valid<T>, Errors> {
        self.validate().map_err(Errors::Validation)?;
        Ok(Valid(self))
    }
}
