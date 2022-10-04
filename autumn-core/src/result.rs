#[derive(Debug, thiserror::Error)]
pub enum AutumnError {
    #[error("Bean does not exist")]
    BeanNotExist,
    #[error("Bean already exists")]
    BeanAlreadyExist,
}

pub type AutumnResult<T> = Result<T, AutumnError>;