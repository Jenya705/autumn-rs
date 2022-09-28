use crate::core::AutumnIdentified;
use crate::descriptor::AutumnBeanInstanceMethodType;

pub struct RunnableMethodType;

impl AutumnBeanInstanceMethodType for RunnableMethodType {
    type Parameters = ();

    type Arguments = ();
}

impl AutumnIdentified for RunnableMethodType {
    type Identifier = Self;
}