use crate::bean::{AutumnBean, AutumnBeanInstance, AutumnBeanInstanceInner, AutumnBeanMap};
use crate::creator::{AutumnBeanCreateData, AutumnBeanCreator, AutumnBeanCreatorInner, AutumnBeanCreatorInnerImpl};
use crate::result::{AutumnError, AutumnResult};

pub struct AutumnContext<'c> {
    bean_states: AutumnBeanMap<AutumnBeanState<'c>>,
}

pub(crate) enum AutumnBeanState<'c> {
    Instance(AutumnBeanInstanceInner<'c>),
    Creator(Box<dyn AutumnBeanCreatorInner<'c>>),
}

impl<'c> AutumnContext<'c> {
    pub fn get_bean_instance<B: AutumnBean>(&self, name: Option<&'static str>) -> AutumnResult<AutumnBeanInstance<'c, B>> {
        self.bean_states.get::<B>()
            .and_then(|value| value.get(name))
            .and_then(|state| match state {
                AutumnBeanState::Instance(ref instance) => Some(unsafe { AutumnBeanInstance::new(instance) }),
                #[warn(unreachable_patterns)]
                _ => None,
            })
            .ok_or(AutumnError::BeanNotExist)
    }

    pub fn add_bean_instance<B: AutumnBean>(&mut self, name: Option<&'static str>, instance: AutumnBeanCreateData<'c, B>) -> AutumnResult<()> {
        let value = self.bean_states.get_mut::<B>();
        match value.get(name) {
            Some(AutumnBeanState::Instance(_)) => Err(AutumnError::BeanAlreadyExist),
            _ => {
                let _ = value.insert(name, AutumnBeanState::Instance(instance.inner));
                Ok(())
            }
        }
    }

    pub async fn compute_bean_creator<B: AutumnBean>(&mut self, name: Option<&'static str>) -> AutumnResult<AutumnBeanInstance<'c, B>> {
        let value = self.bean_states.get_mut::<B>();
        match value.get(name) {
            Some(AutumnBeanState::Instance(instance)) => return Ok(unsafe { AutumnBeanInstance::new(instance) }),
            Some(AutumnBeanState::Creator(_)) => {}
            None => return Err(AutumnError::BeanNotExist)
        };
        let instance = match value.remove(name) {
            Some(AutumnBeanState::Creator(mut creator)) => creator,
            _ => unreachable!(),
        }.create(self).await?;
        let value = self.bean_states.get_mut::<B>();
        let _ = value.insert(name, AutumnBeanState::Instance(instance));
        value.get(name)
            .map(|state| match state {
                AutumnBeanState::Instance(instance) => unsafe { AutumnBeanInstance::new(instance) },
                _ => unreachable!()
            })
            .ok_or(AutumnError::BeanNotExist)
    }

    pub fn add_bean_creator<C: AutumnBeanCreator<'c, B>, B: AutumnBean + 'c>(&mut self, name: Option<&'static str>, creator: C) -> AutumnResult<()> {
        let value = self.bean_states.get_mut::<B>();
        match value.get(name) {
            Some(_) => Err(AutumnError::BeanAlreadyExist),
            _ => {
                let _ = value.insert(name, AutumnBeanState::Creator(Box::new(AutumnBeanCreatorInnerImpl::new(creator))));
                Ok(())
            }
        }
    }
}