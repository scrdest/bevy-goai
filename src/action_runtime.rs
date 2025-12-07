use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::actions::{Action, ScoredAction};

/* Experimental nonsense, remove me */
use crate::actions::ActionContext;
use crate::events::ActionEvent;

#[derive(Debug, EntityEvent)]
struct TestActionEvent{
    /// NOTE: entity is expected to be an ActionTracker here.
    entity: Entity, 
    ctx: ActionContext,
    state: ActionState,
}

impl TestActionEvent {
    fn with_default_context(action_tracker: Entity) -> Self {
        Self { 
            entity: action_tracker,
            ctx: Default::default(), 
            state: ActionState::Running 
        }
    }
}

impl ActionEvent for TestActionEvent {
    fn from_context(context: ActionContext, action_tracker: Entity, state: Option<ActionState>) -> Self {
        Self {
            entity: action_tracker,
            ctx: context,
            state: state.unwrap_or(ActionState::Ready),
        }
    }
}
/* END EXPERIMENTAL NONSENSE */


/// A lifecycle marker for ActionTrackers to indicate what the status of the tracked Action is. 
/// 
/// This is a state machine, more or less, with three layers:
/// 1) Initial
/// 2) Progressed
/// 3) Terminal
/// 
/// *As a general rule*, the progression through those layers should be monotonically non-decreasing, i.e.:
/// - Terminal States should never change at all once reached, 
/// - Progressed states should only become Terminal or different Progressed States, and
/// - Initial states can become any other State.
#[derive(Reflect, Serialize, Deserialize, Debug, Clone, Copy)]
pub enum ActionState {
    // Note that we are NOT implementing Default for this on purpose; 
    // the default state is domain-specific (though probably just Ready most of the time).

    /// Initial state. Planned but not started and cannot start yet - waiting on something (likely other Actions).
    Queued, 
    /// Initial state. Can start now, hadn't done literally anything yet though.
    Ready, 
    
    /// Progressed state. Started but didn't finish yet, will continue.
    Running, 
    /// Progressed state. Started and can continue, but has been put on hold for now.
    Paused, 
    
    /// Terminal state. Did all it was supposed to and is no longer needed, yaaay.
    Succeeded, 
    
    /// Terminal state. 
    /// We gave up due to getting stuck/timeout/etc., naaay. 
    /// Generally implies the action should not be retried as-is 
    /// and may trigger failure callbacks.
    Failed, 
    
    /// Terminal state. 
    /// We gave up because of an 'external' decision that we should, 
    /// and NOT because something was wrong with the execution. 
    /// The Action may well be still valid, we just stopped pursuing it. 
    /// You can think of it as 'Paused, but forever'. 
    Cancelled, 
}

impl ActionState {
    /// A shorthand for checking if an Action is in one of the Initial states (e.g. Ready).
    fn is_initial(&self) -> bool {
        match self {
            Self::Queued => true,
            Self::Ready => true,
            _ => false,
        }
    }

    /// A shorthand for checking if an Action is in one of the Progressed states (e.g. Running).
    fn is_progressed(&self) -> bool {
        match self {
            Self::Running => true,
            Self::Paused => true,
            _ => false,
        }
    }

    /// A shorthand for checking if an Action is in one of the Terminal states (e.g. Succeeded).
    fn is_terminal(&self) -> bool {
        match self {
            Self::Succeeded => true,
            Self::Failed => true,
            Self::Cancelled => true,
            _ => false,
        }
    }

    /// A shorthand for checking if an Action should be processed/'ticked'. If false, we can skip it.
    fn should_process(&self) -> bool {
        match self {
            Self::Ready => true,
            Self::Running => true,
            _ => false,
        }
    }
}


// Action Execution
/// A Component that makes this Entity track an AI Action, i.e. store and expose 
/// data about some Action's execution (e.g. 'Is this still running?' or 'How long ago did it start?') 
/// across (potentially) multiple frames and/or serde operations.
/// 
/// This is the 'master'/'root' marker for tracking Actions - all code within this library will assume 
/// anything without this Component does not track anything, and anything that has any of the extension
/// Component also has this Component.
#[derive(Component, Debug)]
pub struct ActionTracker(ScoredAction);


/// An 'extension' Component for ActionTracker Bundles.
/// 
/// Adds tracking of the AI Entity that 'owns' the tracked Action. 
/// 
/// Bear in mind that Bevy Entities are just Very Fancy IDs; the 
/// Entity tracked by this Component may no longer be valid - you 
/// should verify it yourself where necessary.
/// 
/// This is primarily intended for tying back Actions to AIs performing them 
/// and for cancelling any Actions without an associated AI owner.
#[derive(Component, Debug)]
pub struct ActionTrackerOwningAI {
    owner_ai: Entity
}


/// An 'extension' Component for ActionTracker Bundles.
/// 
/// Adds Action state tracking to the ActionTracker.
/// In this context, state is roughly the 'lifecycle' of an Action: 
/// Pending -> Running -> Terminal (Successful/Failed/Cancelled/etc.).
/// 
/// This is a very handy piece of information. 
/// You can query it for your Actions to do setup work for Pending or 
/// skip processing for Terminal states, in your UI code to visualize
/// the state of the current behavior or provide a 'Cancel Action' button...
/// You might also (correctly) guess it powers some of the Action lifecycle Events.
#[derive(Component, Debug)]
pub struct ActionTrackerState(ActionState);

impl ActionTrackerState {
    fn ready() -> Self {
        Self(ActionState::Ready)
    }

    fn queued() -> Self {
        Self(ActionState::Queued)
    }
}

/// Helper; wraps how we store time for tracking Action runtime timining.
#[derive(Debug)]
pub enum TimeInstantActionTracker {
    Virtual(std::time::Duration),
    Real(std::time::Duration),
    VirtualAndReal((std::time::Duration, std::time::Duration)),
}

/// An 'extension' Component for ActionTracker Bundles.
/// Adds Action time metadata tracking to the ActionTracker for creation time.
/// 
/// This is separate and different from the START time. 
/// An Action 'Start' is when the execution begins, 'Created' is when the task gets planned.
/// 
/// The primary use-case for this is timeouts and cleanup, especially when combined with other 
/// ActionTracker timers - e.g. if an Action was queued up a year ago and it still hasn't finished, 
/// it is most likely a zombie job that should be terminated. 
/// 
/// However, as with all of these, use it as you wish, it's a building block.
#[derive(Component, Debug)]
pub struct ActionTrackerCreationTimer {
    creation_time: TimeInstantActionTracker,
}

/// An 'extension' Component for ActionTracker Bundles. 
/// Adds Action time metadata tracking to the ActionTracker 
/// for Start and End times of the Action's *execution*.
/// 
/// Note that both of these are Option<T>! 
/// 
/// Most obviously, an End time of None means the Action is still pending or running. 
/// You can tell which one it is by the Start time - None means it's pending, i.e. did not start yet.
/// 
/// The primary purpose of this component is for timeouts to kill tasks that
/// got stuck in limbo or did not get cleaned up properly for some reason and the like, 
/// but will almost certainly be handy for UIs and/or Action logic itself as well.
/// 
/// However, as with all of these, use it as you wish, it's a building block.
#[derive(Component, Debug, Default)]
pub struct ActionTrackerRuntimeTimer {
    start_time: Option<TimeInstantActionTracker>,
    end_time: Option<TimeInstantActionTracker>,
}

/// An 'extension' Component for ActionTracker Bundles. 
/// Adds Action time metadata tracking to the ActionTracker 
/// for tracking when the ActionTracker was last 'ticked' 
/// (processed by a system that implements Actually Doing the Action).
/// 
/// The timer value is Optioned; None indicates the ActionTracker has never been ticked.
/// 
/// This Component makes most sense paired with the ActionTrackerTicks Component 
/// which indicates that this ActionTracker CAN ever be ticked at all.
/// 
/// This is not a hard dependency though. In some cases, it might be useful to add and remove 
/// the 'ticking' to an Action dynamically (e.g. as a LOD optimization), so this link is NOT 
/// enforced by the library on purpose.
/// 
/// You are also free to (ab)use this timer by manually updating it in your Observers whenever 
/// you do anything with the tracked Action in a more event-driven Action implementation as well.
/// 
/// The first purpose of this timer is to detect Action garbage - trackers that 
/// had been created, but are not doing any meaningful work to progress to completion.
/// 
/// The second purpose of this timer is to provide a nice way to get time deltas for 
/// 'sparsely' ticked Actions, e.g. event-driven or when reloaded from a savefile.
/// 
/// However, as with all of these, use it as you wish, it's a building block.
#[derive(Component, Debug, Default)]
pub struct ActionTrackerTickTimer {
    last_tick_time: Option<TimeInstantActionTracker>,
}


/// An 'extension' Component for ActionTracker Bundles.
/// 
/// Indicates that this tracker should be processed by a System that   
/// runs some sort of 'tick' logic for the associated Action. 
/// 
/// For example, a MoveTo<pos> Action might move a unit by one grid square, 
/// or one turn's worth of moves, or whatever.
/// 
/// This may be implemented as a function call, a signal emit, both, or whatever else;
/// it's entirely your call as a library user to write your Systems how you want 'em.
/// 
/// This Component is entirely optional - if you don't use it, you can just 
/// catch Events for Action start/end and handle the execution asynchronously 
/// in your game code's 'native' systems (e.g. just set a destination for your 
/// Movement system in an Observer and let it figure out the specifics itself).
#[derive(Component)]
pub struct ActionTrackerTicks;


#[derive(Debug, Clone)]
struct ActionTrackerSpawnConfig {
    track_owner_ai: bool, 
    use_ticker: bool,
    use_create_timer: bool,
    use_runtime_timer: bool,
    use_tick_timer: bool,
}

impl ActionTrackerSpawnConfig {
    fn builder() -> ActionTrackerSpawnConfigBuilder {
        ActionTrackerSpawnConfigBuilder::default()
    }
}

/// Builder pattern for ActionTrackerSpawnConfig
#[derive(Default, Debug, Clone)]
pub struct ActionTrackerSpawnConfigBuilder {
    track_owner_ai: Option<bool>, 
    use_ticker: Option<bool>,
    use_timers: Option<bool>,
    use_create_timer: Option<bool>,
    use_runtime_timer: Option<bool>,
    use_tick_timer: Option<bool>,
}

impl ActionTrackerSpawnConfigBuilder {
    fn build(self) -> ActionTrackerSpawnConfig {
        let track_owner_ai = self.track_owner_ai.unwrap_or(true);
        let use_ticker = self.use_ticker.unwrap_or(false);
        let use_timers = self.use_timers.unwrap_or(false);
        let use_create_timer = self.use_create_timer.unwrap_or(use_timers);
        let use_runtime_timer = self.use_runtime_timer.unwrap_or(use_timers);
        let use_tick_timer = self.use_tick_timer.unwrap_or(use_ticker && use_timers);

        ActionTrackerSpawnConfig {
            track_owner_ai,
            use_ticker,
            use_create_timer,
            use_runtime_timer,
            use_tick_timer,
        }
    }

    fn new() -> Self {
        self::default()
    }

    fn set_track_owner_ai(mut self, val: bool) -> Self {
        self.track_owner_ai = Some(val); self
    }

    fn set_use_ticker(mut self, val: bool) -> Self {
        self.use_ticker = Some(val); self
    }

    fn set_use_timers(mut self, val: bool) -> Self {
        self.use_timers = Some(val); self
    }

    fn set_use_create_timer(mut self, val: bool) -> Self {
        self.use_create_timer = Some(val); self
    }

    fn set_use_runtime_timer(mut self, val: bool) -> Self {
        self.use_runtime_timer = Some(val); self
    }

    fn set_use_tick_timer(mut self, val: bool) -> Self {
        self.use_tick_timer = Some(val); self
    }

    /// Creates a new builder using an existing config as a starting point.
    /// This means the values are preconfigured to match the existing config, 
    /// but you can modify them freely before turning this into a new config.
    fn from_reference_config(config: ActionTrackerSpawnConfig) -> Self {
        Self {
            track_owner_ai: Some(config.track_owner_ai),
            use_ticker: Some(config.use_ticker),
            use_create_timer: Some(config.use_create_timer),
            use_runtime_timer: Some(config.use_runtime_timer),
            use_tick_timer: Some(config.use_tick_timer),
            ..Default::default()
        }
    }
}


impl Into<ActionTrackerSpawnConfig> for ActionTrackerSpawnConfigBuilder {
    fn into(self) -> ActionTrackerSpawnConfig {
        self.build()
    }
}

impl From<ActionTrackerSpawnConfig> for ActionTrackerSpawnConfigBuilder {
    fn from(value: ActionTrackerSpawnConfig) -> Self {
        Self::from_reference_config(value)
    }
}

/// An Event representing some system asking the library to track an Action.
/// You could DIY it, but using this Event should cover typical usecases for ya.
#[derive(EntityEvent)]
pub struct ActionTrackerSpawnRequested {
    /// NOTE: The entity here is intended to be the AIController.
    ///       EntityEvent API sadly doesn't let us rename that for clarity.
    entity: Entity, 
    action: ScoredAction, 
    tracker_config: Option<ActionTrackerSpawnConfig>,
}

impl ActionTrackerSpawnRequested {
    /// Create a new ActionTracker spawn request.
    fn new(entity: Entity, action: ScoredAction, config: Option<ActionTrackerSpawnConfig>) -> Self {
        bevy::log::debug!("Creating a new ActionTrackerSpawnRequested event for {:?}", action);

        Self {
            entity: entity, 
            action: action,
            tracker_config: config,
        }
    }

    /// Create a new ActionTracker spawn request, with whatever defaults the library picked for you. 
    fn with_library_defaults(entity: Entity, action: ScoredAction) -> Self {
        Self::new(entity, action, None)
    }

    /// Create a new ActionTracker spawn request with a specified config.
    fn with_config(entity: Entity, action: ScoredAction, config: ActionTrackerSpawnConfig) -> Self {
        Self::new(entity, action, Some(config))
    }

    /// Create a new ActionTracker spawn request, allowing the config to be built 
    /// by evaluating a specified callback on the ActionTrackerSpawnConfigBuilder.
    /// 
    /// Kind of like the `Option::or_else()` API and the like, lazy evaluation.
    /// 
    /// Mainly a syntax sugar API so you don't have to create a Builder yourself.
    fn with_config_builder(
        entity: Entity, 
        action: ScoredAction, 
        builder: &mut dyn FnMut(ActionTrackerSpawnConfigBuilder) -> Option<ActionTrackerSpawnConfig>,
    ) -> Self {
        Self::new(
            entity, 
            action, 
            builder(ActionTrackerSpawnConfig::builder())
        )
    }
}

/// An Event notifying Observers that a new ActionTracker has been created for an AI.
#[derive(EntityEvent)]
pub struct ActionTrackerSpawnedForTargetAI {
    /// This is the owning AI Entity
    entity: Entity,

    /// This is the created ActionTracker Entity
    action_tracker: Entity,
}

/// Event handler for spawning ActionTrackers for Actions, 
/// triggered by an ActionTrackerSpawnRequested Event.
fn actiontracker_spawn_requested(
    trigger: On<ActionTrackerSpawnRequested>,
    mut commands: Commands,
    game_timer: Res<Time>,
    real_timer: Res<Time<Real>>,
) {
    let event = trigger.event();
    let owner_ai = event.entity;

    let mut tracker = commands.spawn((
        ActionTracker(event.action.clone()),
        ActionTrackerState::ready(),
    ));

    let spawn_config = match &event.tracker_config {
        Some(config) => config,
        None => &ActionTrackerSpawnConfig::builder().build()
    };

    if spawn_config.track_owner_ai {
        tracker.insert(ActionTrackerOwningAI {
            owner_ai: event.entity
        });
    }

    if spawn_config.use_ticker {
        // Add ticking to this ActionTracker.
        // The Component for this is just a marker, pretty trivial.
        tracker.insert(ActionTrackerTicks);
    }

    // Add timing components.
    // 
    // For now we'll use unwrapped elapsed Durations for this as a standard.
    // Real time in particular may span DAYS for reloads, so wrapping it may cause serious artifacts.
    // Duration is u64-based; you may get issues if you leave your game running for 585 billion years.
    if spawn_config.use_create_timer {
        let virtual_spawn_time = game_timer.elapsed();
        let real_spawn_time = real_timer.elapsed();

        tracker.insert(ActionTrackerCreationTimer {
            creation_time: TimeInstantActionTracker::VirtualAndReal((virtual_spawn_time, real_spawn_time))
        });
    }

    if spawn_config.use_runtime_timer {
        // The Action hasn't started yet, so they will both be None for now.
        tracker.insert(ActionTrackerRuntimeTimer::default());
    }

    if spawn_config.use_tick_timer {
        // The Action hasn't been ticked, so starts as None.
        tracker.insert(ActionTrackerTickTimer::default());
    }

    // Send a friendly PSA that we have created this Entity for downstream users to hook into.
    tracker.trigger(|atracker| ActionTrackerSpawnedForTargetAI { 
        entity: owner_ai,
        action_tracker: atracker,
    });
}


/// An Event representing some system asking the library to stop tracking an Action.
/// This will usually happen when the Action has reached some terminal state.
/// You could DIY it, but using this Event should cover typical usecases for ya.
#[derive(EntityEvent)]
pub struct ActionTrackerDespawnRequested {
    entity: Entity, 
}

/// A frankly pretty trivial callback that deletes ActionTrackers that were requested to be cleaned up.
/// As each Tracker is its own unique Entity, this will clean up the whole bundle (including optional modules).
/// 
/// If you want to invoke callbacks on success/failure/etc., this should happen BEFORE this event is raised.
fn actiontracker_despawn_requested(
    event: On<ActionTrackerDespawnRequested>,
    mut commands: Commands,
) {
    let _ = commands.get_entity(event.entity).and_then(|mut e| Ok(e.despawn()));
}

pub fn actiontracker_done_cleanup_system(
    query: Query<(Entity, &ActionTracker, &ActionTrackerState)>, 
    mut commands: Commands, 
) {
    // bevy::log::debug!("Processing ActionTracker cleanup...");

    for (entity, tracker, state) in query.iter() {
        let is_done = match state.0 {
            ActionState::Succeeded => true,
            ActionState::Failed => true,
            ActionState::Cancelled => true,
            _ => false,
        };
        
        bevy::log::debug!("ActionTrackerCleanup: Action {:?} is in state {:?} (done: {:?}).", tracker.0.action.name, state.0, is_done);

        if is_done {
            bevy::log::debug!("ActionTrackerCleanup: Action {:?} finished, cleaning up the Tracker", tracker.0.action);
            commands.trigger(ActionTrackerDespawnRequested {
                entity: entity
            });
        }
    }
}


/// A resource that allows you to specify the global defaults for all ActionTrackers.
/// 
/// If you have a 'house style' for your AI Action implementation, this can save you 
/// a lot of boilerplate.
/// 
/// Note that you can still always override this config for individual exceptional cases.
#[derive(Default, Resource)]
pub struct UserDefaultActionTrackerSpawnConfig {
    config: Option<ActionTrackerSpawnConfig>
}

/// A batteries-included solution for creating ActionTrackers for your Actions.
/// 
/// Event-driven; responds to AiActionPicked events
pub(crate) fn create_tracker_for_picked_action(
    trigger: On<crate::events::AiActionPicked>,
    mut commands: Commands,
    user_default_config_resource: Res<UserDefaultActionTrackerSpawnConfig>,
) {
    let event = trigger.event();

    let action = Action {
        name: event.action_name.clone(),
        action_key: event.action_key.clone(),
        context: event.action_context.clone(),
    };

    let scored_action = ScoredAction {
        action: action,
        score: event.action_score,
    };

    let user_config = user_default_config_resource.config.clone();

    commands.trigger(
        ActionTrackerSpawnRequested::new(
            event.entity,
            scored_action, 
            user_config,
        )
    );
}


#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use bevy::log::LogPlugin;
    use bevy::{app::ScheduleRunnerPlugin, prelude::*};
    use serde_json;
    use crate::actions::ActionTemplate;
    use crate::actionset::ActionSet;
    use crate::ai::AIController;
    use crate::arg_values::ContextValue;
    use crate::utility_concepts::ContextFetcherIdentifier;
    use crate::smart_object::{ActionSetStore, SmartObjects};
    use super::*;

    const TEST_CONTEXT_FETCHER_NAME: &str = "TestCF";

    fn action_tracker_handler(
        mut query: Query<(
            Entity, 
            &ActionTracker, 
            &mut ActionTrackerState, 
            Option<&mut ActionTrackerTickTimer>
        ), With<ActionTrackerTicks>>,
        game_timer: Res<Time>,
        real_timer: Res<Time<Real>>,
        mut commands: Commands,
    ) {
        // User application code - dispatches actual execution Events based on the key in the library Event.
        for (tracker_ent, tracker, state, tick_timer) in query.iter_mut() {
            if !state.0.should_process() {
                bevy::log::debug!("Skipping processing for Action(Tracker) {:?} - {:?}", tracker.0.action.name, state.0);
                continue;
            }

            if let Some(mut tick_timer_included) = tick_timer {
                let current_time_game = game_timer.elapsed();
                let current_time_real = real_timer.elapsed();

                let new_value = TimeInstantActionTracker::VirtualAndReal((
                    current_time_game, current_time_real
                ));

                tick_timer_included.last_tick_time = Some(new_value);
            }
            
            let action_key = &tracker.0.action.action_key;

            match action_key.as_str() {
                "TestAction" => {
                    bevy::log::debug!("Triggering a TestActionEvent...");
                    commands.trigger(TestActionEvent::from_context(
                        tracker.0.action.context.clone(),
                        tracker_ent,
                        Some(state.0),
                    ));
                },
                _ => {}
            }
        }
    }

    fn test_action(
        trigger: On<TestActionEvent>, 
        mut commands: Commands,
    ) {
        let event = trigger.event();
        let tracker = event.entity;

        let tracker_cmds = commands.get_entity(tracker);

        let state = &event.state;
        let maybe_ctx = Some(&event.ctx);

        let json_state = serde_json::ser::to_string(&state);
        let state_name = json_state.unwrap();
        bevy::log::debug!("Current state is {}", state_name);

        let self_name: Option<&String> = maybe_ctx.map(|ctx| ctx.get("this").unwrap().try_into().unwrap());
        bevy::log::debug!("Self name is {:?}", self_name);

        let context_mapping = maybe_ctx.map(|ctx| ctx.get(&state_name)).flatten();

        let new: ActionState = match context_mapping {
            None => None,
            Some(cv) => {
                let clone_val = cv.clone();
                let cvstring: String = clone_val.try_into().unwrap();
                let unjsond = serde_json::de::from_str(&cvstring).unwrap();
                bevy::log::debug!("Current unjsond is {:?}", unjsond);
                unjsond
            }
        }.unwrap();

        bevy::log::debug!("New state is {:?}", new);

        match tracker_cmds {
            Err(err) => bevy::log::debug!("ActionTracker does not exist: {:?}", err),
            Ok(mut cmds) => { 
                bevy::log::debug!("Updating the ActionTracker {:?} state to new value {:?}", tracker, new);
                cmds.insert(ActionTrackerState(new)); 
            },
        }
    }

    fn test_context_fetcher() -> Vec<crate::actions::ActionContext> {
        let mut context: HashMap<String, ContextValue> = HashMap::with_capacity(3);
        // As an artifact of how we use JSON serde, we need to add escaped quotes around strings here.
        context.insert("\"Ready\"".to_string(), "\"Running\"".to_string().into());
        context.insert("\"Running\"".to_string(), "\"Failed\"".to_string().into());
        context.insert("\"Failed\"".to_string(), "\"Failed\"".to_string().into());
        context.insert("this".to_string(), TEST_CONTEXT_FETCHER_NAME.to_string().into());
        Vec::from([context])
    }

    fn setup_test_entity(
        mut commands: Commands,
        mut actionset_store: ResMut<ActionSetStore>,
    ) {
        let test_actions = [
            ActionTemplate  {
                name: "TestAction".to_string(),
                context_fetcher_name: ContextFetcherIdentifier(TEST_CONTEXT_FETCHER_NAME.to_string()),
                considerations: Vec::from([]),
                priority: 1.,
                action_key: "TestAction".to_string(),
            }
        ];

        let test_actionset = ActionSet {
            name: "TestActionSet".to_string(),
            actions: Vec::from(test_actions)
        };

        actionset_store.map_by_name.insert(test_actionset.name.to_owned(), test_actionset);

        let new_controller = AIController::default();
        let new_sos = SmartObjects {
            actionset_refs: Vec::from(["TestActionSet".to_string()])
        };

        let spawned = commands.spawn((
            new_controller,
        ));

        let ai_id = spawned.id();

        commands.trigger(crate::events::AiDecisionRequested { 
            entity: ai_id,  
            smart_objects: Some(new_sos)
        });
    }

    fn setup_default_action_tracker_config(
        mut config_res: ResMut<UserDefaultActionTrackerSpawnConfig>
    ) {
        let new_config = 
            ActionTrackerSpawnConfigBuilder::new()
            .set_use_ticker(true)
            .set_use_timers(false)
        ;
        config_res.config = Some(new_config.build());
    }

    #[test]
    fn test_run_action() {
        let mut app = App::new();

        app
        .add_plugins((
            // MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(std::time::Duration::from_millis(200))),
            MinimalPlugins.set(ScheduleRunnerPlugin::run_once()),
            LogPlugin { 
                level: bevy::log::Level::DEBUG, 
                custom_layer: |_| None, 
                filter: "wgpu=error,bevy_render=info,bevy_ecs=info".to_string(),
                fmt_layer: |_| None,
            }
        ))
        .init_resource::<UserDefaultActionTrackerSpawnConfig>()
        .init_resource::<ActionSetStore>()
        .register_function_with_name(TEST_CONTEXT_FETCHER_NAME, test_context_fetcher)
        .add_systems(Startup, setup_test_entity)
        .add_systems(Startup, setup_default_action_tracker_config)
        .add_observer(create_tracker_for_picked_action)
        .add_observer(actiontracker_spawn_requested)
        .add_observer(actiontracker_despawn_requested)
        .add_observer(crate::decision_loop::decision_process)
        .add_systems(Update, action_tracker_handler)
        .add_observer(test_action)
        .add_systems(PostUpdate, actiontracker_done_cleanup_system)
        ;

        app.run();
    }
}


