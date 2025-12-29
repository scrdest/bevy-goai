use bevy::prelude::*;
// use bevy::reflect::{FromType, Reflect};
use crate::actions::ActionContext;


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
    pub action_context: crate::types::ActionContextRef,

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

        let wrapped_ctx = std::sync::Arc::new(action_context);

        Self {
            entity: ai_owner,
            action_key: action_key,
            action_name: action_name,
            action_context: wrapped_ctx,
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
    pub entity: Entity,
    pub smart_objects: Option<crate::smart_object::SmartObjects>,
}


#[cfg(test)]
mod tests {
    use bevy::log::LogPlugin;
    use bevy::{app::ScheduleRunnerPlugin, prelude::*};
    use super::*;
    use crate::ai::AIController;

    #[derive(Debug, Default, Event)]
    struct TestActionEvent;

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

        let ctx2 = std::sync::Arc::new(ctx2);

        commands.trigger(AiActionPicked {
            action_name: "TestAction".into(),
            action_key: "TestActionEvent".into(),
            action_context: ctx2,
            action_score: 1.,
            entity: entity.into(),
        });
    }

    fn dispatch_events(
        trigger: On<AiActionPicked>,
        mut commands: Commands,
    ) {
        let evt = trigger.event();
        let actionkey = evt.action_key.as_str();

        match actionkey {
            "TestActionEvent" => { commands.trigger(TestActionEvent) }
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



