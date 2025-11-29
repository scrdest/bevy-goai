use bevy::prelude::*;
use bevy::reflect::{FromType, Reflect};
use crate::actions::ActionContext;
use crate::action_runtime::ActionPickedEvent;


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


#[cfg(test)]
mod tests {
    use bevy::log::LogPlugin;
    use bevy::{app::ScheduleRunnerPlugin, prelude::*};
    use super::*;
    use crate::ai::AIController;

    #[derive(Debug, Default, Event)]
    struct TestActionEvent(ActionContext);

    impl ActionEvent for TestActionEvent {
        fn from_context(context: ActionContext) -> Self {
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

        commands.trigger(ActionPickedEvent {
            action_name: "TestAction".into(),
            action_key: "TestActionEvent".into(),
            context: ctx2,
            source_ai: entity,
        });
    }

    fn dispatch_events(
        trigger: Trigger<ActionPickedEvent>,
        mut commands: Commands,
    ) {
        let evt = trigger.event();
        let actionkey = evt.action_key.as_str();

        match actionkey {
            "TestActionEvent" => { commands.trigger(TestActionEvent(evt.context.to_owned())) }
            _ => { panic!("Unrecognized Action Key: {}", actionkey) }
        };
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
        .add_observer(dispatch_events)
        .add_observer(handle_event)
        ;

        app.run();
    }
}



