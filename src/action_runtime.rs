use std::collections::{HashMap};
use bevy::prelude::*;
use bevy::reflect::DynamicMap;
use crate::actions::{DynFuncName};

pub enum ActionState {
    Running,
    Succeeded,
    Failed,
}

type ActionContext = DynamicMap;

trait Action: Send + Sync {
    fn tick(&mut self, context: ActionContext) -> ActionState;
}


// Context & Action Execution
#[derive(Component)]
pub struct CurrentAction {
    template: DynFuncName,
    context: HashMap<String, Box<dyn Reflect>>, // Dynamic context
    state: ActionState, 
}
