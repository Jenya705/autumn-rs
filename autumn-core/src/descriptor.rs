use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ptr::{NonNull, null_mut};
use std::sync::Arc;
use crate::core::{AutumnBean, AutumnContext, AutumnContextReference, AutumnIdentified, AutumnResult};

pub trait AutumnBeanInstanceMethodType {
    type Parameters: AutumnIdentified;

    type Arguments: AutumnIdentified;
}

#[repr(transparent)]
pub struct AutumnBeanInstanceMethodDescriptor<'a, MT: AutumnBeanInstanceMethodType>(&'a AutumnBeanInstanceMethodDescriptorInner, PhantomData<MT>);

struct AutumnBeanInstanceMethodDescriptorInner {
    parameters: NonNull<()>,
    method: fn(
        AutumnBeanInstanceMethodReference,
        *mut (), // Arguments will be deleted after, because it is allocated in the heap of calling function (possibly null)
    ) -> AutumnResult<()>,
}

#[derive(Default)]
pub struct AutumnBeanInstanceDescriptor {
    methods: HashMap<TypeId, Vec<AutumnBeanInstanceMethodDescriptorInner>>,
}

#[derive(Clone, Copy)]
pub struct AutumnBeanInstanceMethodReference {
    mutable: bool,
    bean: NonNull<()>,
    context: NonNull<()>,
}

impl AutumnBeanInstanceMethodReference {
    pub fn new_mut<B: AutumnBean>(bean: &B, context: &mut AutumnContext) -> Self {
        Self {
            mutable: true,
            bean: unsafe { NonNull::new_unchecked(bean as *const B as *mut ()) },
            context: unsafe { NonNull::new_unchecked(context as *mut AutumnContext as *mut ()) },
        }
    }

    pub fn new<B: AutumnBean>(bean: &B, context: &AutumnContext) -> Self {
        Self {
            mutable: false,
            bean: unsafe { NonNull::new_unchecked(bean as *const B as *mut ()) },
            context: unsafe { NonNull::new_unchecked(context as *const AutumnContext as *mut ()) },
        }
    }
}

impl<'a, MT: AutumnBeanInstanceMethodType> AutumnBeanInstanceMethodDescriptor<'a, MT> {
    fn new(inner: &'a AutumnBeanInstanceMethodDescriptorInner) -> Self {
        Self(inner, PhantomData)
    }

    pub fn get_parameters(&self) -> &MT::Parameters {
        unsafe { &*(self.0.parameters.as_ptr() as *const MT::Parameters) }
    }

    pub fn execute(&self, reference: AutumnBeanInstanceMethodReference, mut arguments: Option<MT::Arguments>) -> AutumnResult<()> {
        (self.0.method)(
            reference,
            arguments.as_mut()
                .map(|args| args as *mut MT::Arguments as *mut ())
                .unwrap_or(null_mut()),
        )
    }
}

impl AutumnBeanInstanceDescriptor {
    pub fn empty_arc() -> Arc<Self> {
        Arc::new(Self::empty())
    }

    pub fn empty() -> Self {
        Default::default()
    }

    pub fn get_method_descriptors<MT: AutumnBeanInstanceMethodType>(&self) -> Option<impl Iterator<Item=AutumnBeanInstanceMethodDescriptor<MT>>> {
        self.methods.get(&TypeId::of::<<MT::Parameters as AutumnIdentified>::Identifier>())
            .map(|descriptors| descriptors.iter()
                .map(|descriptor| AutumnBeanInstanceMethodDescriptor::<MT>::new(descriptor))
            )
    }
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

    pub fn into_unknown_ref<'a, 'c, B>(self) -> (&'c B, AutumnContextReference<'a, 'c>) {
        let mutable = self.mutable;
        let (bean, context) = unsafe { self.unsafe_mut() };
        (bean, match mutable {
            true => AutumnContextReference::Mutable(context),
            false => AutumnContextReference::Immutable(context),
        })
    }
}