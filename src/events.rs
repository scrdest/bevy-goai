use bevy::prelude::*;
use bevy::reflect::{FromType, Reflect};
use crate::actions::ActionContext;
use crate::action_runtime::ActionState;


#[derive(Event)]
pub enum GoaiActionEvent {
    Zero(ActionState, Option<ActionContext>),
    One(ActionState, Option<ActionContext>),
    Two(ActionState, Option<ActionContext>),
    Three(ActionState, Option<ActionContext>),
    Four(ActionState, Option<ActionContext>),
    Five(ActionState, Option<ActionContext>),
    Six(ActionState, Option<ActionContext>),
    Seven(ActionState, Option<ActionContext>),
    Eight(ActionState, Option<ActionContext>),
    Nine(ActionState, Option<ActionContext>),
    Ten(ActionState, Option<ActionContext>),
    Eleven(ActionState, Option<ActionContext>),
    Twelve(ActionState, Option<ActionContext>),
    Thirteen(ActionState, Option<ActionContext>),
    Fourteen(ActionState, Option<ActionContext>),
    Fifteen(ActionState, Option<ActionContext>),
    Sixteen(ActionState, Option<ActionContext>),
}

pub(crate) type GoaiActionEventId = u32;

impl GoaiActionEvent {
    pub(crate) fn from_id_and_context(id: GoaiActionEventId, context: Option<ActionContext>) -> Result<Self, String> {
        match id {
            0 => Ok(Self::Zero(ActionState::Running, context)),
            1 => Ok(Self::One(ActionState::Running, context)),
            2 => Ok(Self::Two(ActionState::Running, context)),
            3 => Ok(Self::Three(ActionState::Running, context)),
            4 => Ok(Self::Four(ActionState::Running, context)),
            5 => Ok(Self::Five(ActionState::Running, context)),
            6 => Ok(Self::Six(ActionState::Running, context)),
            7 => Ok(Self::Seven(ActionState::Running, context)),
            8 => Ok(Self::Eight(ActionState::Running, context)),
            9 => Ok(Self::Zero(ActionState::Running, context)),
            10 => Ok(Self::Ten(ActionState::Running, context)),
            11 => Ok(Self::Eleven(ActionState::Running, context)),
            12 => Ok(Self::Twelve(ActionState::Running, context)),
            13 => Ok(Self::Thirteen(ActionState::Running, context)),
            14 => Ok(Self::Fourteen(ActionState::Running, context)),
            15 => Ok(Self::Fifteen(ActionState::Running, context)),
            16 => Ok(Self::Sixteen(ActionState::Running, context)),
            _ => Err(format!("Id {} outside of supported range (max: 16)", id))
        }
    }

    pub(crate) fn get_state(&self) -> &ActionState {
        match self {
            Self::Zero(state, _) => state,
            Self::One(state, _) => state,
            Self::Two(state, _) => state,
            Self::Three(state, _) => state,
            Self::Four(state, _) => state,
            Self::Five(state, _) => state,
            Self::Six(state, _) => state,
            Self::Seven(state, _) => state,
            Self::Eight(state, _) => state,
            Self::Nine(state, _) => state,
            Self::Ten(state, _) => state,
            Self::Eleven(state, _) => state,
            Self::Twelve(state, _) => state,
            Self::Thirteen(state, _) => state,
            Self::Fourteen(state, _) => state,
            Self::Fifteen(state, _) => state,
            Self::Sixteen(state, _) => state,
        }
    }

    pub(crate) fn get_context(&self) -> &Option<ActionContext> {
        match self {
            Self::Zero(_, ctx) => ctx,
            Self::One(_, ctx) => ctx,
            Self::Two(_, ctx) => ctx,
            Self::Three(_, ctx) => ctx,
            Self::Four(_, ctx) => ctx,
            Self::Five(_, ctx) => ctx,
            Self::Six(_, ctx) => ctx,
            Self::Seven(_, ctx) => ctx,
            Self::Eight(_, ctx) => ctx,
            Self::Nine(_, ctx) => ctx,
            Self::Ten(_, ctx) => ctx,
            Self::Eleven(_, ctx) => ctx,
            Self::Twelve(_, ctx) => ctx,
            Self::Thirteen(_, ctx) => ctx,
            Self::Fourteen(_, ctx) => ctx,
            Self::Fifteen(_, ctx) => ctx,
            Self::Sixteen(_, ctx) => ctx,
        }
    }
}


#[derive(Clone)]
pub struct ReflectIntoEvent {
    builder: fn(GoaiActionEventId, ActionContext) -> GoaiActionEvent
}

impl ReflectIntoEvent {
    pub fn builder(&self, id: GoaiActionEventId, context: ActionContext) -> GoaiActionEvent {
        (self.builder)(id, context)
    }
}

impl<E: Reflect + ActionEvent + Default> FromType<E> for ReflectIntoEvent {
    fn from_type() -> Self {
        Self {
            builder: |id, ctx| GoaiActionEvent::from_id_and_context(id, Some(ctx)).unwrap(),
        }
    }
}

pub trait ActionEventFactory: Reflect {
    type AsEvent: ActionEvent;

    fn to_action_event(&self) -> Self::AsEvent;
}

pub trait ActionEvent: Reflect {
    type AsEvent: Event + Reflect;

    fn from_context(context: ActionContext) -> Self::AsEvent;

    fn from_context_reflect(context: ActionContext) -> Box<Self::AsEvent> {
        let base = Self::from_context(context);
        Box::new(base)
    }
}


/// Marker trait for ActionEvents that do not make use of the Context, 
/// and can therefore be implemented cheaply using the type's Default implementation. 
/// This is meant for either: 
/// 
/// (1) events that store no data at all (i.e. empty structs deriving Event), or 
/// (2) events that only store stuff in safely defaultable containers to be filled in later.
pub trait ContextFreeActionEvent: ActionEvent + Default {
    fn from_context(_context: ActionContext) -> Self {
        Self::default()
    }
}


