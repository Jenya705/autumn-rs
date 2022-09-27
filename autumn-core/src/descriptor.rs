use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::Arc;
use ref_cast::RefCast;
use crate::core::{AutumnBean, AutumnContext, AutumnContextReference, AutumnIdentified, AutumnResult};

pub trait AutumnBeanInstanceMethodType {
    type Parameters: AutumnIdentified;

    type Arguments: AutumnIdentified;
}

pub struct AutumnBeanInstanceMethodCall<'a, 'c, MT: AutumnBeanInstanceMethodType> {
    pub(crate) reference: AutumnBeanInstanceMethodReference,
    pub descriptor: &'a AutumnBeanInstanceMethodDescriptor<MT>,
    pub(crate) pt: PhantomData<&'c ()>,
}

#[repr(transparent)]
#[derive(RefCast)]
pub struct AutumnBeanInstanceMethodDescriptor<MT: AutumnBeanInstanceMethodType>(
    AutumnBeanInstanceMethodDescriptorInner, PhantomData<MT>,
);

#[derive(Clone, Copy)]
pub struct AutumnBeanInstanceMethodDescriptorInner {
    parameters: NonNull<()>,
    method: fn(
        &AutumnBeanInstanceMethodReference,
        *const (), // Arguments is shared between calls and allocated in the heap of calling function
    ) -> AutumnResult<()>,
}

#[derive(Default)]
pub struct AutumnBeanInstanceDescriptor {
    pub(crate) methods: HashMap<TypeId, Vec<AutumnBeanInstanceMethodDescriptorInner>>,
}

#[derive(Clone, Copy)]
pub struct AutumnBeanInstanceMethodReference {
    pub(crate) mutable: bool,
    pub(crate) bean: NonNull<()>,
    pub(crate) context: NonNull<()>,
}

#[derive(Default)]
pub(crate) struct AutumnContextDescriptor {
    pub(crate) methods: HashMap<TypeId, Vec<(NonNull<()>, AutumnBeanInstanceMethodDescriptorInner)>>,
}

impl<'a, 'c, MT: AutumnBeanInstanceMethodType> AutumnBeanInstanceMethodCall<'a, 'c, MT> {
    pub fn call(&self, arguments: &MT::Arguments) -> AutumnResult<()> {
        (self.descriptor.0.method)(
            &self.reference,
            arguments as *const MT::Arguments as *const (),
        )
    }
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

impl<'a, MT: AutumnBeanInstanceMethodType> AutumnBeanInstanceMethodDescriptor<MT> {
    pub fn new(parameters: Box<MT::Parameters>, method: fn(&AutumnBeanInstanceMethodReference, *const ()) -> AutumnResult<()>) -> Self {
        Self(
            AutumnBeanInstanceMethodDescriptorInner {
                parameters: unsafe { NonNull::new_unchecked(Box::into_raw(parameters) as *mut ()) },
                method,
            },
            PhantomData,
        )
    }

    pub fn get_parameters(&self) -> &MT::Parameters {
        unsafe { &*(self.0.parameters.as_ptr() as *const MT::Parameters) }
    }
}

impl AutumnBeanInstanceDescriptor {
    pub fn empty_arc() -> Arc<Self> {
        Arc::new(Self::empty())
    }

    pub fn empty() -> Self {
        Default::default()
    }

    pub fn get_method_descriptors<MT: AutumnBeanInstanceMethodType>(&self) -> Option<impl Iterator<Item=&AutumnBeanInstanceMethodDescriptor<MT>>> {
        self.methods.get(&TypeId::of::<<MT::Parameters as AutumnIdentified>::Identifier>())
            .map(|descriptors| descriptors.iter()
                .map(|descriptor| AutumnBeanInstanceMethodDescriptor::<MT>::ref_cast(descriptor))
            )
    }

    pub fn add_method_descriptor<MT: AutumnBeanInstanceMethodType>(&mut self, method_descriptor: AutumnBeanInstanceMethodDescriptor<MT>) {
        self.methods.entry(TypeId::of::<<MT::Parameters as AutumnIdentified>::Identifier>())
            .or_insert_with(|| Vec::new())
            .push(method_descriptor.0)
    }
}

impl AutumnBeanInstanceMethodReference {
    unsafe fn unsafe_mut<'a, 'c, B>(&self) -> (&'c B, &'a mut AutumnContext<'c>) {
        let bean = &*(self.bean.as_ptr() as *const B);
        let context = &mut *(self.context.as_ptr() as *mut AutumnContext<'c>);
        (bean, context)
    }

    pub fn as_mut<'a, 'c, B>(&self) -> Option<(&'c B, &'a mut AutumnContext<'c>)> {
        match self.mutable {
            true => Some(unsafe { self.unsafe_mut() }),
            false => None,
        }
    }

    pub fn as_ref<'a, 'c, B>(&self) -> (&'c B, &'a AutumnContext<'c>) {
        let (bean, context) = unsafe { self.unsafe_mut() };
        (bean, context)
    }

    pub fn as_unknown_ref<'a, 'c, B>(&self) -> (&'c B, AutumnContextReference<'a, 'c>) {
        let mutable = self.mutable;
        let (bean, context) = unsafe { self.unsafe_mut() };
        (bean, match mutable {
            true => AutumnContextReference::Mutable(context),
            false => AutumnContextReference::Immutable(context),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use crate::core::{AutumnBean, AutumnContext, AutumnIdentified, AutumnResult};
    use crate::descriptor::{AutumnBeanInstanceDescriptor, AutumnBeanInstanceMethodDescriptor, AutumnBeanInstanceMethodReference, AutumnBeanInstanceMethodType};

    #[derive(Debug)]
    struct PrintlnBean(String);

    impl AutumnBean for PrintlnBean {}

    impl AutumnIdentified for PrintlnBean {
        type Identifier = PrintlnBean;
    }

    impl AutumnBeanInstanceMethodType for PrintlnBean {
        type Parameters = ();

        type Arguments = ();
    }

    pub fn println_task(reference: &AutumnBeanInstanceMethodReference, _parameters: *const ()) -> AutumnResult<()> {
        println!("{}", reference.as_ref::<PrintlnBean>().0.0);
        Ok(())
    }

    #[test]
    fn descriptor_test() {
        let mut context = AutumnContext::new();
        context.add_bean_instance(
            Box::new(PrintlnBean("hello, world".to_string())),
            None,
            Arc::new({
                let mut descriptor = AutumnBeanInstanceDescriptor::empty();
                descriptor.add_method_descriptor(AutumnBeanInstanceMethodDescriptor::<PrintlnBean>::new(
                    Box::new(()),
                    println_task,
                ));
                descriptor
            }),
        ).unwrap();
        context.get_methods::<PrintlnBean>()
            .for_each(|call| call.call(&()).unwrap())
    }
}