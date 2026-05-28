/// Return ownership of data to caller on fail
pub struct ErrorAnd<T, E>(pub T, pub E);
