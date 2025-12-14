#[derive(Debug)]
pub enum DynResolutionError {
    UnexpectedType(String),
    NotInRegistry(String)
}
