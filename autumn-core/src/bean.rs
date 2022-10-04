use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::marker::PhantomData;
use crate::ptr::UnknownPointer;

pub trait AutumnBean: AutumnIdentified + Sync {}

pub trait AutumnIdentified {
    type Identifier: Any;
}

pub fn autumn_id<T: AutumnIdentified>() -> TypeId {
    TypeId::of::<T::Identifier>()
}

#[repr(transparent)]
pub struct AutumnBeanInstance<'c, T> {
    pub(crate) inner: AutumnBeanInstanceInner<'c>,
    _marker: PhantomData<T>,
}

pub(crate) struct AutumnBeanInstanceInner<'c> {
    pub(crate) pointer: UnknownPointer,
    _marker: PhantomData<&'c ()>,
}

#[repr(transparent)]
pub struct AutumnBeanMap<T> {
    map: HashMap<TypeId, AutumnBeanMapValue<T>>,
}

pub struct AutumnBeanMapValue<T> {
    unnamed: Option<T>,
    named: HashMap<&'static str, T>,
}

impl<'c, T> AutumnBeanInstance<'c, T> {
    pub(crate) unsafe fn new<'a>(inner: &'a AutumnBeanInstanceInner<'c>) -> &'a Self {
        &*(inner as *const AutumnBeanInstanceInner<'c> as *const () as *const Self)
    }

    pub fn get(&self) -> &'c T {
        unsafe { self.inner.pointer.get().cast().as_ref() }
    }
}

impl<'c> AutumnBeanInstanceInner<'c> {
    pub(crate) fn new(pointer: UnknownPointer) -> Self {
        Self {
            pointer,
            _marker: PhantomData,
        }
    }
}

impl<T> AutumnBeanMap<T> {
    pub fn get_mut<B: AutumnIdentified>(&mut self) -> &mut AutumnBeanMapValue<T> {
        self.map.entry(autumn_id::<B>()).or_insert_with(|| AutumnBeanMapValue::new())
    }

    pub fn get<B: AutumnIdentified>(&self) -> Option<&AutumnBeanMapValue<T>> {
        self.map.get(&autumn_id::<B>())
    }
}

impl<T> AutumnBeanMapValue<T> {
    pub(crate) fn new() -> Self {
        Self {
            unnamed: None,
            named: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: Option<&'static str>, value: T) -> Option<T> {
        match name {
            Some(name) => self.named.insert(name, value),
            None => self.unnamed.replace(value),
        }
    }

    pub fn get(&self, name: Option<&'static str>) -> Option<&T> {
        match name {
            Some(name) => self.named.get(name),
            None => self.unnamed.as_ref(),
        }
    }

    pub fn get_mut(&mut self, name: Option<&'static str>) -> Option<&mut T> {
        match name {
            Some(name) => self.named.get_mut(name),
            None => self.unnamed.as_mut(),
        }
    }

    pub fn remove(&mut self, name: Option<&'static str>) -> Option<T> {
        match name {
            Some(name) => self.named.remove(name),
            None => self.unnamed.take()
        }
    }
}