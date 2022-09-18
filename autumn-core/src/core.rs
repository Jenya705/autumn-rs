use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

pub trait AutumnBean: Any + Sync + Send + Debug {
    fn get_name(&self) -> Option<&'static str>;
}

#[derive(Debug, thiserror::Error)]
pub enum AutumnError {
    #[error("Bean {0:?} already exists")]
    BeanAlreadyExist(Arc<dyn AutumnBean>),
    #[error("Bean {0:?} with name {1} does not exist")]
    BeanNotExist(TypeId, String),
}

pub struct AutumnContext {
    beans: HashMap<TypeId, AutumnBeanContainer<Arc<dyn Any + Send + Sync>>>,
}

struct AutumnBeanContainer<T> {
    unnamed: Option<T>,
    names: HashMap<&'static str, T>,
}

impl AutumnContext {
    pub fn new() -> Self {
        Self {
            beans: HashMap::new(),
        }
    }

    pub fn add_bean_instance<B: AutumnBean>(&mut self, bean: Arc<B>) -> Result<(), AutumnError> {
        let type_id = bean.as_ref().type_id();
        if !self.beans.contains_key(&type_id) {
            self.beans.insert(type_id.clone(), AutumnBeanContainer::new());
        }
        // code in bottom guaranties that by this key exists bean container
        let bean_container = self.beans.get_mut(&type_id).unwrap();
        let bean_name = bean.get_name();
        match bean_container.insert(bean.clone(), bean_name) {
            true => Ok(()),
            false => Err(AutumnError::BeanAlreadyExist(bean))
        }
    }

    pub fn get_bean_instance<B: AutumnBean>(&self) -> Option<Arc<B>> {
        self.beans.get(&TypeId::of::<B>())
            .and_then(|bean_container| bean_container.unnamed.as_ref())
            .map(|arc| arc.clone().downcast().unwrap())
    }

    pub fn get_named_bean_instance<B: AutumnBean>(&self, name: &str) -> Option<Arc<B>> {
        self.beans.get(&TypeId::of::<B>())
            .and_then(|bean_container| bean_container.names.get(name))
            .map(|arc| arc.clone().downcast().unwrap())
    }
}

impl<T> AutumnBeanContainer<T> {
    fn new() -> Self {
        Self {
            unnamed: None,
            names: HashMap::new(),
        }
    }

    fn insert(&mut self, object: T, name: Option<&'static str>) -> bool {
        match name {
            Some(name) => match self.names.contains_key(name) {
                true => false,
                false => {
                    self.names.insert(name, object);
                    true
                }
            }
            None => match self.unnamed {
                Some(_) => false,
                None => {
                    self.unnamed = Some(object);
                    true
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use super::*;

    #[derive(Debug)]
    struct SimpleBean {
        name: Option<&'static str>,
        some_counter: Mutex<i32>,
    }

    impl AutumnBean for SimpleBean {
        fn get_name(&self) -> Option<&'static str> {
            self.name.clone()
        }
    }

    #[test]
    fn bean_get_test() {
        let mut context = AutumnContext::new();
        context.add_bean_instance(Arc::new(SimpleBean {
            name: None, some_counter: Mutex::new(0)
        })).unwrap();
        *context.get_bean_instance::<SimpleBean>().unwrap().some_counter.lock().unwrap() += 1;
        assert_eq!(*context.get_bean_instance::<SimpleBean>().unwrap().some_counter.lock().unwrap(), 1);
    }
}