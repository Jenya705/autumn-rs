use std::any::Any;
use std::borrow::Cow;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug)]
pub enum BeanParameter {
    String(Cow<'static, str>),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Slice(Cow<'static, [BeanParameter]>),
    Any(Box<dyn Any>),
}

#[derive(Debug, thiserror::Error)]
pub enum InitializationError {
    #[error("Require bean: {}")]
    RequireBean(Cow<'static, str>),
    #[error("{}")]
    Any(#[from] anyhow::Error),
}

pub trait Service: Bean + BeanInitializer<Self> {}

#[crate::async_trait]
pub trait BeanInitializer<B: Bean> {
    async fn initialize<C: Context>(context: C) -> Result<Box<B>, InitializationError>;
}

pub trait Bean: Parameterable + Any + Sync + Send {
    fn get_name(&self) -> Option<&'static str>;
}

pub trait Parameterable {
    fn get_parameter(&self, name: &str) -> Option<BeanParameter>;
}

pub trait Context {
    fn get_bean<T: Bean>(&self) -> Option<Arc<T>>;

    fn get_named_bean<T: Bean>(&self, name: &str) -> Option<Arc<T>>;
}

pub trait ContextBuilder: Context {
    fn add_bean_initializer<B: Bean, I: BeanInitializer<B>, C: Context>(&mut self, initializer: I);

    // TODO. Think about making bean: Box<dyn Bean>. Because type of this bean could be known from itself
    fn add_bean_instance<B: Bean>(&mut self, bean: Box<B>);

    /// Builds more optimized context which should be used in runtime
    fn build<C: Context>(self) -> C;
}