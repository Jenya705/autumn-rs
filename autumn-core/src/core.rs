use std::any::{Any, type_name, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

pub trait AutumnBean: Any + Sync + Send + Debug {}

pub trait AutumnBeanCreator<B: AutumnBean>: 'static {
    fn create_instance(self, context: &mut AutumnContext) -> AutumnResult<Arc<B>>;
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
pub struct AutumnContext {
    parent: Option<Arc<AutumnContext>>,
    bean_sources: HashMap<TypeId, AutumnBeanContainer<AutumnBeanSource>>,
}

type AutumnBeanCreatorFn = Box<dyn FnOnce(&mut AutumnContext) -> AutumnResult<()>>;

enum AutumnBeanSource {
    Creator(AutumnBeanCreatorFn),
    Instance(Arc<dyn Any + Send + Sync>),
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

impl AutumnContext {
    pub fn new() -> Self {
        Default::default()
    }

    fn get_mut_bean_container<B: AutumnBean>(&mut self) -> &mut AutumnBeanContainer<AutumnBeanSource> {
        let type_id = TypeId::of::<B>();
        if !self.bean_sources.contains_key(&type_id) {
            self.bean_sources.insert(type_id.clone(), AutumnBeanContainer::new());
        }
        self.bean_sources.get_mut(&type_id).unwrap()
    }

    fn get_parent_bean_instance<B: AutumnBean>(&self, name: Option<&'static str>) -> AutumnResult<Arc<B>> {
        self.parent.as_ref()
            .map(|parent| parent.get_bean_instance(name))
            .unwrap_or_else(|| Err(AutumnError::bean_not_exist::<B>(name)))
    }

    pub fn add_bean_instance<B: AutumnBean>(&mut self, bean: Arc<B>, name: Option<&'static str>) -> AutumnResult<()> {
        let bean_container = self.get_mut_bean_container::<B>();
        let bean_source = bean_container.get(&name);
        match bean_source {
            Some(AutumnBeanSource::Creator(_)) | None => Ok(bean_container.replace(AutumnBeanSource::Instance(bean), &name)),
            Some(AutumnBeanSource::Instance(_)) => Err(AutumnError::BeanAlreadyExist)
        }
    }

    pub fn add_bean_creator<B: AutumnBean, C: AutumnBeanCreator<B>>(&mut self, creator: C, name: Option<&'static str>) -> AutumnResult<()> {
        let bean_container = self.get_mut_bean_container::<B>();
        let bean_name_fn = name.clone();
        let bean_creator_fn = move |autumn_context: &mut AutumnContext| {
            let instance = creator.create_instance(autumn_context)?;
            autumn_context.add_bean_instance(instance, bean_name_fn)
        };
        let bean_source = bean_container.get(&name);
        match bean_source.is_none() {
            true => Ok(bean_container.replace(AutumnBeanSource::Creator(Box::new(bean_creator_fn)), &name)),
            false => Err(AutumnError::BeanAlreadyExist)
        }
    }

    pub fn get_bean_instance<B: AutumnBean>(&self, name: Option<&'static str>) -> AutumnResult<Arc<B>> {
        self.bean_sources.get(&TypeId::of::<B>())
            .and_then(|bean_container| match bean_container.get(&name) {
                Some(AutumnBeanSource::Instance(ref instance)) => Some(instance),
                Some(AutumnBeanSource::Creator(_)) | None => None,
            })
            .map(|arc| Ok(arc.clone().downcast().unwrap()))
            .unwrap_or_else(|| self.get_parent_bean_instance(name))
    }

    pub fn compute_bean_instance<B: AutumnBean>(&mut self, name: Option<&'static str>) -> AutumnResult<Arc<B>> {
        let creator = match self.bean_sources.get_mut(&TypeId::of::<B>()) {
            Some(bean_container) => {
                let bean_source = bean_container.get(&name);
                match bean_source {
                    Some(AutumnBeanSource::Creator(_)) => match bean_container.remove(&name) {
                        Some(AutumnBeanSource::Creator(creator)) => creator,
                        _ => unreachable!(),
                    }
                    Some(AutumnBeanSource::Instance(instance)) => return Ok(instance.clone().downcast().unwrap()),
                    None => return self.get_parent_bean_instance(name)
                }
            }
            None => return self.get_parent_bean_instance(name),
        };
        creator(self)?;
        self.get_bean_instance(name)
    }

    pub fn compute_all_bean_instances(&mut self) -> AutumnResult<()> {
        let mut creators = Vec::new();
        for (type_id, bean_container) in &mut self.bean_sources {
            let names = bean_container.names.iter().map(|(name, _)| *name).collect::<Vec<&str>>();
            if let Some(AutumnBeanSource::Creator(_)) = bean_container.unnamed {
                creators.push((type_id.clone(), None));
            }
            for name in names {
                if let Some(AutumnBeanSource::Creator(_)) = bean_container.names.get(name) {
                    creators.push((type_id.clone(), Some(name)));
                }
            }
        }
        for (type_id, name) in creators {
            let creator = match self.bean_sources.get_mut(&type_id) {
                Some(bean_container) => {
                    match bean_container.get(&name) {
                        Some(AutumnBeanSource::Creator(_)) => match bean_container.remove(&name) {
                            Some(AutumnBeanSource::Creator(creator)) => creator,
                            _ => unreachable!()
                        },
                        _ => continue
                    }
                }
                None => continue
            };
            creator(self)?;
        }
        Ok(())
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

    struct SimpleBeanCreator {}

    impl AutumnBean for SimpleBean {}

    impl AutumnBeanCreator<SimpleBean> for SimpleBeanCreator {
        fn create_instance(self, _context: &mut AutumnContext) -> AutumnResult<Arc<SimpleBean>> {
            Ok(Arc::new(SimpleBean {
                some_counter: Mutex::new(32),
            }))
        }
    }

    #[derive(Debug)]
    struct NeedItSelfBean;

    struct NeedItSelfBeanCreator(Option<&'static str>);

    impl AutumnBean for NeedItSelfBean {}

    impl AutumnBeanCreator<NeedItSelfBean> for NeedItSelfBeanCreator {
        fn create_instance(self, context: &mut AutumnContext) -> AutumnResult<Arc<NeedItSelfBean>> {
            context.compute_bean_instance::<NeedItSelfBean>(self.0)?;
            Ok(Arc::new(NeedItSelfBean))
        }
    }

    #[test]
    fn bean_get_test() {
        let mut context = AutumnContext::new();
        context.add_bean_instance(Arc::new(SimpleBean {
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
}