use crate::errors::Errors;

pub struct Input<T: validator::Validate>(pub T);
pub struct Valid<T>(pub T);

impl<T: validator::Validate> TryFrom<Input<T>> for Valid<T> {
    type Error = Errors;
    fn try_from(Input(t): Input<T>) -> Result<Self, Self::Error> {
        t.validate().map_err(Errors::Validation)?;
        Ok(Valid(t))
    }
}
