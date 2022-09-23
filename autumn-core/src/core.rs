use std::any::{Any, type_name, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::Arc;

pub trait AutumnBean: Sync + Send + Debug {
    /// This identifier will be used to get [std::any::TypeId] from
    type Identifier: Any;
}

pub trait AutumnBeanCreator<'a, B: AutumnBean + 'a>: 'static {
    fn create_instance(self, context: &mut AutumnContext<'a>) -> AutumnResult<Box<B>>;
}

#[derive(Debug, thiserror::Error)]
pub enum AutumnError {
    #[error("Bean already exists")]
    BeanAlreadyExist,
    #[error("Bean {0} with name {1:?} does not exist")]
    BeanNotExist(&'static str, Option<String>),
}

pub type AutumnResult<T> = Result<T, AutumnError>;

#[derive(Default)]
pub struct AutumnContext<'a> {
    parent: Option<Arc<AutumnContext<'a>>>,
    bean_sources: HashMap<TypeId, AutumnBeanContainer<AutumnBeanSource<'a>>>,
}

pub enum AutumnContextReference<'a, 'c> {
    Mutable(&'a mut AutumnContext<'c>),
    Immutable(&'a AutumnContext<'c>),
}

type AutumnBeanCreatorFn = Box<dyn FnOnce(*mut ()) -> AutumnResult<()>>;

enum AutumnBeanSource<'a> {
    Creator(AutumnBeanCreatorFn, PhantomData<&'a ()>),
    Instance(AutumnBeanInstance<'a>),
}

struct AutumnBeanInstance<'a> {
    ptr: NonNull<()>,
    _pa: PhantomData<&'a ()>,
}

struct AutumnBeanContainer<T> {
    unnamed: Option<T>,
    names: HashMap<&'static str, T>,
}

impl AutumnError {
    pub fn bean_not_exist<B: AutumnBean>(name: Option<&'static str>) -> Self {
        Self::BeanNotExist(type_name::<B>(), name.map(|str| str.to_string()))
    }
}

impl<'a> AutumnContext<'a> {
    pub fn new() -> Self {
        Default::default()
    }

    fn get_mut_bean_container<B: AutumnBean>(&mut self) -> &mut AutumnBeanContainer<AutumnBeanSource<'a>> {
        let type_id = TypeId::of::<B::Identifier>();
        if !self.bean_sources.contains_key(&type_id) {
            self.bean_sources.insert(type_id.clone(), AutumnBeanContainer::new());
        }
        self.bean_sources.get_mut(&type_id).unwrap()
    }

    fn get_parent_bean_instance<B: AutumnBean>(&self, name: Option<&'static str>) -> AutumnResult<&'a B> {
        self.parent.as_ref()
            .map(|parent| parent.get_bean_instance(name))
            .unwrap_or_else(|| Err(AutumnError::bean_not_exist::<B>(name)))
    }

    fn call_creator(&mut self, creator: AutumnBeanCreatorFn) -> AutumnResult<()> {
        creator(self as *mut Self as *mut ())
    }

    pub fn add_bean_instance<B: AutumnBean + 'a>(&mut self, bean: Box<B>, name: Option<&'static str>) -> AutumnResult<()> {
        let bean_container = self.get_mut_bean_container::<B>();
        let bean_source = bean_container.get(&name);
        match bean_source {
            Some(AutumnBeanSource::Creator(..)) | None => Ok(bean_container.replace(AutumnBeanSource::Instance(AutumnBeanInstance::new(bean)), &name)),
            Some(AutumnBeanSource::Instance(_)) => Err(AutumnError::BeanAlreadyExist)
        }
    }

    pub fn add_bean_creator<B: AutumnBean + 'a, C: AutumnBeanCreator<'a, B>>(&mut self, creator: C, name: Option<&'static str>) -> AutumnResult<()> {
        let bean_container = self.get_mut_bean_container::<B>();
        let bean_name_fn = name.clone();
        // Safety. This function will be executed only for this context, so its lifetime will be 'a
        let bean_creator_fn = move |autumn_context: *mut ()| unsafe {
            let autumn_context = (autumn_context as *mut AutumnContext<'a>).as_mut().unwrap();
            let instance = creator.create_instance(autumn_context)?;
            autumn_context.add_bean_instance(instance, bean_name_fn)
        };
        let bean_source = bean_container.get(&name);
        match bean_source.is_none() {
            true => Ok(bean_container.replace(AutumnBeanSource::Creator(Box::new(bean_creator_fn), PhantomData), &name)),
            false => Err(AutumnError::BeanAlreadyExist)
        }
    }

    pub fn get_bean_instance<B: AutumnBean>(&self, name: Option<&'static str>) -> AutumnResult<&'a B> {
        self.bean_sources.get(&TypeId::of::<B::Identifier>())
            .and_then(|bean_container| match bean_container.get(&name) {
                Some(AutumnBeanSource::Instance(ref instance)) => Some(Ok(unsafe { instance.get::<B>() })),
                Some(AutumnBeanSource::Creator(..)) | None => None,
            })
            .unwrap_or_else(|| self.get_parent_bean_instance(name))
    }

    pub fn compute_bean_instance<B: AutumnBean>(&mut self, name: Option<&'static str>) -> AutumnResult<&'a B> {
        let creator = match self.bean_sources.get_mut(&TypeId::of::<B::Identifier>()) {
            Some(bean_container) => {
                let bean_source = bean_container.get(&name);
                match bean_source {
                    Some(AutumnBeanSource::Creator(..)) => match bean_container.remove(&name) {
                        Some(AutumnBeanSource::Creator(creator, _)) => creator,
                        _ => unreachable!(),
                    }
                    Some(AutumnBeanSource::Instance(instance)) => return Ok(unsafe { instance.get::<B>() }),
                    None => return self.get_parent_bean_instance(name)
                }
            }
            None => return self.get_parent_bean_instance(name),
        };
        self.call_creator(creator)?;
        self.get_bean_instance(name)
    }

    pub fn compute_all_bean_instances(&mut self) -> AutumnResult<()> {
        let mut creators = Vec::new();
        for (type_id, bean_container) in &mut self.bean_sources {
            let names = bean_container.names.iter().map(|(name, _)| *name).collect::<Vec<&str>>();
            if let Some(AutumnBeanSource::Creator(..)) = bean_container.unnamed {
                creators.push((type_id.clone(), None));
            }
            for name in names {
                if let Some(AutumnBeanSource::Creator(..)) = bean_container.names.get(name) {
                    creators.push((type_id.clone(), Some(name)));
                }
            }
        }
        for (type_id, name) in creators {
            let creator = match self.bean_sources.get_mut(&type_id) {
                Some(bean_container) => {
                    match bean_container.get(&name) {
                        Some(AutumnBeanSource::Creator(..)) => match bean_container.remove(&name) {
                            Some(AutumnBeanSource::Creator(creator, _)) => creator,
                            _ => unreachable!()
                        },
                        _ => continue
                    }
                }
                None => continue
            };
            self.call_creator(creator)?;
        }
        Ok(())
    }
}

impl<'a, 'c> AutumnContextReference<'a, 'c> {
    pub fn get_mut<'b>(&'b mut self) -> Option<&'b mut AutumnContext<'c>> {
        match self {
            Self::Mutable(mutable) => Some(*mutable),
            Self::Immutable(_) => None,
        }
    }

    pub fn get_ref<'b>(&'b mut self) -> &'b AutumnContext<'c> {
        match self {
            Self::Mutable(mutable) => *mutable,
            Self::Immutable(immutable) => *immutable,
        }
    }

    pub fn get_bean_instance<B: AutumnBean + 'c>(&mut self, name: Option<&'static str>) -> AutumnResult<&'a B> {
        match self {
            Self::Mutable(mutable) => mutable.compute_bean_instance(name),
            Self::Immutable(immutable) => immutable.get_bean_instance(name),
        }
    }
}

impl<'a> AutumnBeanInstance<'a> {
    pub fn new<B>(bean: Box<B>) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(Box::into_raw(bean) as *mut ()) },
            _pa: PhantomData,
        }
    }

    pub unsafe fn get<B>(&self) -> &'a B {
        &*(self.ptr.as_ptr() as *const B)
    }
}

impl<T> AutumnBeanContainer<T> {
    fn new() -> Self {
        Self {
            unnamed: None,
            names: HashMap::new(),
        }
    }

    fn get(&self, name: &Option<&'static str>) -> Option<&T> {
        match name {
            Some(ref name) => self.names.get(name),
            None => self.unnamed.as_ref()
        }
    }

    fn remove(&mut self, name: &Option<&'static str>) -> Option<T> {
        match name {
            Some(ref name) => self.names.remove(name),
            None => self.unnamed.take()
        }
    }

    fn replace(&mut self, object: T, name: &Option<&'static str>) {
        match name {
            Some(name) => { self.names.insert(name, object); }
            None => self.unnamed = Some(object),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use super::*;

    #[derive(Debug)]
    struct SimpleBean {
        some_counter: Mutex<i32>,
    }

    struct SimpleBeanCreator;

    impl AutumnBean for SimpleBean {
        type Identifier = SimpleBean;
    }

    impl AutumnBeanCreator<'_, SimpleBean> for SimpleBeanCreator {
        fn create_instance(self, _context: &mut AutumnContext) -> AutumnResult<Box<SimpleBean>> {
            Ok(Box::new(SimpleBean {
                some_counter: Mutex::new(32),
            }))
        }
    }

    #[derive(Debug)]
    struct NeedItSelfBean;

    struct NeedItSelfBeanCreator(Option<&'static str>);

    impl AutumnBean for NeedItSelfBean {
        type Identifier = NeedItSelfBean;
    }

    impl AutumnBeanCreator<'_, NeedItSelfBean> for NeedItSelfBeanCreator {
        fn create_instance(self, context: &mut AutumnContext) -> AutumnResult<Box<NeedItSelfBean>> {
            context.compute_bean_instance::<NeedItSelfBean>(self.0)?;
            Ok(Box::new(NeedItSelfBean))
        }
    }

    #[derive(Debug)]
    struct DependedBean<'a>(&'a SimpleBean);

    struct DependedBeanCreator;

    impl<'a> AutumnBean for DependedBean<'a> {
        type Identifier = DependedBean<'static>;
    }

    impl<'a> AutumnBeanCreator<'a, DependedBean<'a>> for DependedBeanCreator {
        fn create_instance(self, context: &mut AutumnContext<'a>) -> AutumnResult<Box<DependedBean<'a>>> {
            Ok(Box::new(DependedBean(
                context.compute_bean_instance::<SimpleBean>(None)?
            )))
        }
    }

    #[test]
    fn bean_get_test() {
        let mut context = AutumnContext::new();
        context.add_bean_instance(Box::new(SimpleBean {
            some_counter: Mutex::new(0),
        }), None).unwrap();
        *context.get_bean_instance::<SimpleBean>(None).unwrap().some_counter.lock().unwrap() += 1;
        assert_eq!(*context.get_bean_instance::<SimpleBean>(None).unwrap().some_counter.lock().unwrap(), 1);
    }

    #[test]
    fn bean_compute_test() {
        let mut context = AutumnContext::new();
        context.add_bean_creator(SimpleBeanCreator {}, None).unwrap();
        assert_eq!(context.get_bean_instance::<SimpleBean>(None).is_err(), true);
        assert_eq!(*context.compute_bean_instance::<SimpleBean>(None).unwrap().some_counter.lock().unwrap(), 32);
    }

    #[test]
    fn bean_compute_all_test() {
        let mut context = AutumnContext::new();
        context.add_bean_creator(SimpleBeanCreator {}, None).unwrap();
        context.compute_all_bean_instances().unwrap();
        assert_eq!(*context.get_bean_instance::<SimpleBean>(None).unwrap().some_counter.lock().unwrap(), 32);
        assert_eq!(*context.compute_bean_instance::<SimpleBean>(None).unwrap().some_counter.lock().unwrap(), 32);
    }

    #[test]
    fn bean_recursion_get_test() {
        let mut context = AutumnContext::new();
        context.add_bean_creator(NeedItSelfBeanCreator(None), None).unwrap();
        assert_eq!(context.compute_all_bean_instances().is_err(), true)
    }

    #[test]
    fn bean_dependencies_test() {
        let mut context = AutumnContext::new();
        context.add_bean_creator(DependedBeanCreator, None).unwrap();
        context.add_bean_creator(SimpleBeanCreator, None).unwrap();
        assert_eq!(
            context.compute_bean_instance::<DependedBean>(None).unwrap().0 as *const SimpleBean,
            context.compute_bean_instance::<SimpleBean>(None).unwrap() as *const SimpleBean,
        )
    }
}