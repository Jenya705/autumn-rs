use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ptr::NonNull;
use crate::core::{AutumnContext, AutumnResult};

#[repr(transparent)]
pub struct AutumnBeanInstanceMethodDescriptor<'a, B: Any>(&'a AutumnBeanInstanceMethodDescriptorInner, PhantomData<B>);

struct AutumnBeanInstanceMethodDescriptorInner {
    parameters: NonNull<()>,
    method: fn(AutumnBeanInstanceMethodReference) -> AutumnResult<()>,
}

pub struct AutumnBeanInstanceDescriptor {
    methods: HashMap<TypeId, Vec<AutumnBeanInstanceMethodDescriptorInner>>,
}

pub struct AutumnBeanInstanceMethodReference {
    mutable: bool,
    bean: NonNull<()>,
    context: NonNull<()>,
}

impl AutumnBeanInstanceMethodReference {
    unsafe fn unsafe_mut<'a, 'c, B>(self) -> (&'c B, &'a mut AutumnContext<'c>) {
        let bean = &*(self.bean.as_ptr() as *const B);
        let context = &mut *(self.context.as_ptr() as *mut AutumnContext<'c>);
        (bean, context)
    }

    pub fn into_mut<'a, 'c, B>(self) -> Option<(&'c B, &'a mut AutumnContext<'c>)> {
        match self.mutable {
            true => Some(unsafe { self.unsafe_mut() }),
            false => None,
        }
    }

    pub fn into_ref<'a, 'c, B>(self) -> (&'c B, &'a AutumnContext<'c>) {
        let (bean, context) = unsafe { self.unsafe_mut() };
        (bean, context)
    }
}