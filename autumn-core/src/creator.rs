use std::marker::PhantomData;
use std::ptr::NonNull;
use crate::bean::{AutumnBean, AutumnBeanInstanceInner};
use crate::context::AutumnContext;
use crate::ptr::UnknownPointer;
use crate::result::AutumnResult;

#[crate::async_trait]
pub trait AutumnBeanCreator<'c, B: AutumnBean>: 'c + Send {
    async fn create(&mut self, context: &mut AutumnContext<'c>) -> AutumnResult<AutumnBeanCreateData<'c, B>>;
}

pub struct AutumnBeanCreateData<'c, B> {
    pub(crate) inner: AutumnBeanInstanceInner<'c>,
    _marker: PhantomData<B>,
}

#[crate::async_trait]
pub(crate) trait AutumnBeanCreatorInner<'c>: 'c + Send {
    async fn create(&mut self, context: &mut AutumnContext<'c>) -> AutumnResult<AutumnBeanInstanceInner<'c>>;
}

pub(crate) struct AutumnBeanCreatorInnerImpl<'c, B: AutumnBean, C: AutumnBeanCreator<'c, B>> {
    creator: C,
    _marker: PhantomData<&'c B>,
}

impl<'c, B> AutumnBeanCreateData<'c, B> {
    pub fn new(bean: Box<B>) -> Self {
        Self {
            inner: AutumnBeanInstanceInner::new(UnknownPointer::new(unsafe { NonNull::new_unchecked(Box::into_raw(bean)) })),
            _marker: PhantomData,
        }
    }
}

impl<'c, B: AutumnBean, C: AutumnBeanCreator<'c, B>> AutumnBeanCreatorInnerImpl<'c, B, C> {
    pub(crate) fn new(creator: C) -> Self {
        Self {
            creator,
            _marker: PhantomData,
        }
    }
}

#[crate::async_trait]
impl<'c, B: AutumnBean, C: AutumnBeanCreator<'c, B>> AutumnBeanCreatorInner<'c> for AutumnBeanCreatorInnerImpl<'c, B, C> {
    async fn create(&mut self, context: &mut AutumnContext<'c>) -> AutumnResult<AutumnBeanInstanceInner<'c>> {
        self.creator.create(context).await.map(|instance| instance.inner)
    }
}