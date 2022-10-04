use crate::bean::{AutumnBean, AutumnBeanInstance, AutumnBeanInstanceInner, AutumnBeanMap};
use crate::result::{AutumnError, AutumnResult};

pub struct AutumnContext<'c> {
    bean_states: AutumnBeanMap<AutumnBeanState<'c>>,
}

pub(crate) enum AutumnBeanState<'c> {
    Instance(AutumnBeanInstanceInner<'c>),
}

impl<'c> AutumnContext<'c> {
    pub fn get_bean_instance<B: AutumnBean>(&self, name: Option<&'static str>) -> AutumnResult<&AutumnBeanInstance<'c, B>> {
        self.bean_states.get::<B>()
            .and_then(|value| value.get(name))
            .and_then(|state| match state {
                AutumnBeanState::Instance(ref instance) => Some(unsafe { AutumnBeanInstance::new(instance) }),
                #[warn(unreachable_patterns)]
                _ => None,
            })
            .ok_or(AutumnError::BeanNotExist)
    }

    pub fn add_bean_instance<B: AutumnBean>(&mut self, name: Option<&'static str>, instance: AutumnBeanInstance<'c, B>) -> AutumnResult<()> {
        let value = self.bean_states.get_mut::<B>();
        match value.get(name).is_some() {
            true => Err(AutumnError::BeanAlreadyExist),
            false => {
                let _ = value.insert(name, AutumnBeanState::Instance(instance.inner));
                Ok(())
            }
        }
    }
}