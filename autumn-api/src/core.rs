use std::any::Any;
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub enum BeanParameter {
    String(Cow<'static, str>),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Any(Box<dyn Any>),
}

pub type BeanId = &'static str;

pub trait Service: Bean {
    fn from_context<C: Context>(context: &C);
}

pub trait Bean {
    fn get_id(&self) -> BeanId;

    fn get_name(&self) -> Option<&'static str>;

    fn get_parameter(&self, name: &str) -> Option<BeanParameter>;
}

pub trait Context {
    fn get_bean<T: Bean>(&self) -> Option<T>;

    fn get_named_bean<T: Bean>(&self, name: &str) -> Option<T>;
}