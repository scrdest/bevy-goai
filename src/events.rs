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

pub trait ActionEvent: Event {
    fn from_context(context: ActionContext) -> Self;
}


/// Marker trait intended for types implementing ActionEvent.
/// Indicates that the ActionEvent does not use any Context values. 
/// 
/// This implies the construction of ActionEvents for this type is trivial; 
/// all ActionEvents must be constructable using just the Context, and for 
/// these the Context is irrelevant as well (so we can create them at will).
/// 
/// In particular, if the type implements this AND Default, the Default constructor
/// is guaranteed to work as a constructor for a triggerable Event as well.
pub trait IsContextFree {}


/// Marker trait for ActionEvents that do not make use of the Context, 
/// and can therefore be implemented cheaply using the type's Default implementation. 
/// (blanket-implemented for all matching types of ActionEvent types).
/// This is meant for either: 
/// 
/// (1) events that store no data at all (i.e. empty structs deriving Event), or 
/// (2) events that only store stuff in safely defaultable containers to be filled in later.
pub trait ContextFreeActionEvent: ActionEvent + IsContextFree + Default {
    fn from_context(_context: ActionContext) -> Self {
        Self::default()
    }
}

impl<T: ActionEvent + IsContextFree + Default> ContextFreeActionEvent for T {}


/// This trait represents 'something that can raise ActionEvents'.
/// 
/// It can be used in systems generic over ActionEvents that handle triggering 
/// the actual Events (using Commands access), using whichever communication channels 
/// you might wish to implement it with (e.g. a buffer backed by a Vec<ActionContext>).
/// 
/// This is necessary due to Bevy's Events being both non-dyn-safe and non-Reflect; 
/// there is no golden path to raise an arbitrary, type-erased Event; you need to 
/// know the Event type statically, even with all the Reflect magic.
pub trait ActionEventFactory {
    type AsEvent: ActionEvent;

    fn get_input_contexts(&mut self) -> Vec<ActionContext>;
    fn to_action_event(&self, ctx: &ActionContext) -> Self::AsEvent;

    fn run(&mut self) -> Vec<Self::AsEvent> {
        let contexts = self.get_input_contexts();
        let event_stream = contexts.iter().map(|ctx| self.to_action_event(ctx));
        event_stream.collect()
    }
}


fn raise_action_events<AEF: ActionEventFactory + Resource>(
    mut factory: ResMut<AEF>,
    mut commands: Commands,
) {
    let events = factory.run();
    events.into_iter().for_each(|evt| {
        commands.trigger(evt)
    });
}



#[cfg(test)]
mod tests {
    use bevy::log::LogPlugin;
    use bevy::{app::ScheduleRunnerPlugin, prelude::*};
    use super::*;

    #[derive(Debug, Default, Event)]
    struct TestActionEvent(ActionContext);

    impl ActionEvent for TestActionEvent {
        fn from_context(context: ActionContext) -> Self {
            Self(context)
        }
    }

    #[derive(Resource, Default, Reflect)]
    struct TestActionEventFactory {
        queue: Vec<ActionContext>
    }

    impl ActionEventFactory for TestActionEventFactory {
        type AsEvent = TestActionEvent;

        fn get_input_contexts(&mut self) -> Vec<ActionContext> {
            let out = self.queue.to_vec();
            self.queue = Vec::new();
            out
        }
    
        fn to_action_event(&self, ctx: &ActionContext) -> Self::AsEvent {
            TestActionEvent(ctx.to_owned())
        }
    }

    fn setup_test_entity(
        mut commands: Commands,
    ) {
        let mut ctx2 = ActionContext::new();
        ctx2.insert("foo".into(), 1.into());
        ctx2.insert("bar".into(), 2.into());

        let factory = TestActionEventFactory { 
            queue: Vec::from([
                ActionContext::new(),
                ctx2,
            ])
        };
        commands.insert_resource(factory);
    }

    fn handle_event(
        trigger: Trigger<TestActionEvent>
    ) {
        let evt = trigger.event();
        bevy::log::debug!("Processing event {:?}", evt);
    }

    #[test]
    fn test_run_action() {
        let mut app = App::new();

        app
        .add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_once()),
            LogPlugin { 
                level: bevy::log::Level::DEBUG, 
                custom_layer: |_| None, 
                filter: "wgpu=error,bevy_render=info,bevy_ecs=info".to_string(),
            }
        ))
        .add_systems(Startup, setup_test_entity)
        .add_systems(Update, raise_action_events::<TestActionEventFactory>)
        .add_observer(handle_event)
        ;

        app.run();
    }
}



