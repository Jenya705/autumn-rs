use std::sync::Mutex;
use crate::bean::{AutumnBean, AutumnIdentified};
use crate::context::AutumnContext;
use crate::creator::{AutumnBeanCreateData, AutumnBeanCreator};
use crate::result::{AutumnError, AutumnResult};

struct CountService {
    counter: Mutex<i32>,
}

struct CountServiceCreator;

impl AutumnBean for CountService {}

impl AutumnIdentified for CountService {
    type Identifier = Self;
}

impl CountService {
    pub fn new() -> Self {
        Self {
            counter: Mutex::new(0),
        }
    }

    pub fn increment(&self) -> i32 {
        let mut counter = self.counter.lock().unwrap();
        let value = *counter;
        *counter += 1;
        value
    }
}

#[crate::async_trait]
impl<'c> AutumnBeanCreator<'c, CountService> for CountServiceCreator {
    async fn create(&mut self, _context: &mut AutumnContext<'c>) -> AutumnResult<AutumnBeanCreateData<'c, CountService>> {
        Ok(AutumnBeanCreateData::new(Box::new(CountService::new())))
    }
}

#[test]
fn add_get_bean_test() {
    let mut context = AutumnContext::new();
    context.add_bean_instance(None, AutumnBeanCreateData::new(Box::new(CountService::new()))).unwrap();
    assert_eq!(context.get_bean_instance::<CountService>(None).unwrap().get().increment(), 0);
    assert_eq!(context.get_bean_instance::<CountService>(None).unwrap().get().increment(), 1);
}

#[tokio::test]
async fn add_compute_bean_creator_test() {
    let mut context = AutumnContext::new();
    context.add_bean_creator(None, CountServiceCreator).unwrap();
    assert_eq!(context.compute_bean_instance::<CountService>(None).await.unwrap().get().increment(), 0);
    assert_eq!(context.compute_bean_instance::<CountService>(None).await.unwrap().get().increment(), 1);
}

#[test]
fn try_reinsert_bean_test() {
    let mut context = AutumnContext::new();
    context.add_bean_instance(None, AutumnBeanCreateData::new(Box::new(CountService::new()))).unwrap();
    assert_eq!(context.add_bean_instance(None, AutumnBeanCreateData::new(Box::new(CountService::new()))).unwrap_err(), AutumnError::BeanAlreadyExist);
}

#[tokio::test]
async fn recursive_dependency_test() {
    struct RecursiveBean;
    struct RecursiveBeanCreator;
    impl AutumnBean for RecursiveBean {}
    impl AutumnIdentified for RecursiveBean { type Identifier = RecursiveBean; }
    #[crate::async_trait]
    impl<'c> AutumnBeanCreator<'c, RecursiveBean> for RecursiveBeanCreator {
        async fn create(&mut self, context: &mut AutumnContext<'c>) -> AutumnResult<AutumnBeanCreateData<'c, RecursiveBean>> {
            context.compute_bean_instance::<RecursiveBean>(None).await.map(|_| AutumnBeanCreateData::new(Box::new(RecursiveBean)))
        }
    }

    let mut context = AutumnContext::new();
    context.add_bean_creator(None, RecursiveBeanCreator).unwrap();
    assert_eq!(context.compute_bean_instance::<RecursiveBean>(None).await.map(|_| panic!()).unwrap_err(), AutumnError::BeanNotExist);
}