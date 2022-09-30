use std::any::{Any, TypeId};
use std::marker::PhantomData;
use std::ptr::NonNull;

pub trait AutumnBean: AutumnIdentified + Sync {}

pub trait AutumnIdentified {
    type Identifier: Any;
}

pub const fn autumn_id<T: AutumnIdentified>() -> TypeId {
    TypeId::of::<T::Identifier>()
}

#[repr(transparent)]
pub struct AutumnBeanInstance<'c, T>(
    pub(crate) AutumnBeanInstanceInner<'c>,
    pub(crate) PhantomData<T>,
);

#[derive(Clone, Copy)]
pub(crate) struct AutumnBeanInstanceInner<'c> {
    pub(crate) instance_ptr: NonNull<()>,
}

impl<'c> AutumnBeanInstanceInner<'c> {
    pub const unsafe fn get_mut<T: AutumnBean>(&mut self) -> &'c mut T {
        unsafe { self.instance_ptr.cast().as_mut() }
    }

    pub const unsafe fn get_ref<T: AutumnBean>(&self) -> &'c T {
        unsafe { self.instance_ptr.cast().as_ref() }
    }
}

impl<'c, T> AutumnBeanInstance<'c, T> {
    pub const fn get_mut(&mut self) -> &'c mut T {
        unsafe { self.0.get_mut() }
    }

    pub const fn get_ref(&self) -> &'c T {
        unsafe { self.0.get_ref() }
    }
}