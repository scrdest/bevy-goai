use bevy::prelude::*;
// use bevy::reflect::{FromType, Reflect};
use crate::actions::ActionContext;
use crate::action_runtime::ActionState;


pub trait ActionEvent: Event {
    fn from_context(context: ActionContext, action_tracker: Entity, state: Option<ActionState>) -> Self;
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


/// An Event that signals the decision engine picked the new best Action
/// for a specific AI Entity and provides details about it (abstract ID, 
/// context, etc.).
/// 
/// Primarily expected to be raised by the decision_process() System 
/// and listened to by consumers for remapping into more Action-specific logic
/// (e.g. raising an Event for a *specific* Action implementation).
#[derive(EntityEvent, Debug)]
pub struct AiActionPicked {
    /// The AI that picked this Action for execution. 
    pub entity: Entity,

    /// Identifier for the handling event (e.g. "GoTo"). 
    /// This is effectively a link to the *implementation* of the action. 
    pub action_key: crate::types::ActionKey,

    /// Human-readable primary identifier; one action_key may handle distinct action_names 
    /// (e.g. action_key "GoTo" may cover action_names "Walk", "Run", "Flee", etc.).
    /// In other words, this is what this action represents *semantically*, and is less likely
    /// to change for technical purposes.
    pub action_name: String,

    /// The Context of the Action, i.e. the static input(s) we scored against.
    pub action_context: ActionContext,

    /// The Utility score; this is so that we can decide whether to possibly 
    /// override this with a higher-priority Action later on.
    pub action_score: crate::types::ActionScore,
}

impl AiActionPicked {
    pub fn new(
        ai_owner: Entity,
        action_key: crate::types::ActionKey,
        action_name: String,
        action_context: ActionContext,
        action_score: crate::types::ActionScore,
    ) -> Self {
        
        bevy::log::debug!(
            "Creating a new AiActionPicked event for {:?} with key {:?} ({:?})",
            ai_owner,
            action_key,
            action_name
        );

        Self {
            entity: ai_owner,
            action_key: action_key,
            action_name: action_name,
            action_context: action_context,
            action_score: action_score,
        }
    }
}


/// Supporting Event for triggering a decision_process() for an AI.
/// Raised whenever an active AI starts a tick without an Action.
/// 
/// Should generally NOT be raised more than once per Entity per tick 
/// or you are likely running the same calculation multiple times.
#[derive(EntityEvent)]
pub struct AiDecisionRequested {
    pub(crate) entity: Entity,
    pub smart_objects: Option<crate::smart_object::SmartObjects>,
}


#[cfg(test)]
mod tests {
    use bevy::log::LogPlugin;
    use bevy::{app::ScheduleRunnerPlugin, prelude::*};
    use super::*;
    use crate::ai::AIController;

    #[derive(Debug, Default, Event)]
    struct TestActionEvent(ActionContext);

    impl ActionEvent for TestActionEvent {
        fn from_context(context: ActionContext, action_tracker: Entity, action_state: Option<ActionState>) -> Self {
            Self(context)
        }
    }

    fn setup_test_entity(
        mut commands: Commands,
    ) {
        let entity_cmds = commands.spawn(
            AIController::default()
        );

        let entity = entity_cmds.id();

        let mut ctx2 = ActionContext::new();
        ctx2.insert("foo".into(), 1.into());
        ctx2.insert("bar".into(), 2.into());

        commands.trigger(AiActionPicked {
            action_name: "TestAction".into(),
            action_key: "TestActionEvent".into(),
            action_context: ctx2,
            action_score: 1.,
            entity: entity,
        });
    }

    fn dispatch_events(
        trigger: On<AiActionPicked>,
        mut commands: Commands,
    ) {
        let evt = trigger.event();
        let actionkey = evt.action_key.as_str();

        match actionkey {
            "TestActionEvent" => { commands.trigger(TestActionEvent(evt.action_context.to_owned())) }
            _ => { panic!("Unrecognized Action Key: {}", actionkey) }
        };
    }

    fn handle_event(
        trigger: On<TestActionEvent>
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
                fmt_layer: |_| None,
            }
        ))
        .add_systems(Startup, setup_test_entity)
        .add_observer(dispatch_events)
        .add_observer(handle_event)
        ;

        app.run();
    }
}



