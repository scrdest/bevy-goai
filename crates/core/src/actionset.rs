
use std::collections::HashMap;
use std::time::Duration;

use bevy::asset::{AssetLoader, LoadContext, io::Reader, Asset, io::AssetSourceBuilder};
use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crate::actions::{ActionTemplate};


#[derive(Asset, Reflect, Serialize, Deserialize, Debug, Clone)]
pub struct ActionSet {
    pub name: String,
    pub actions: Vec<ActionTemplate>,
}

// Asset loader
#[derive(Default)]
pub struct ActionSetLoader;

impl AssetLoader for ActionSetLoader {
    type Asset = ActionSet;
    type Settings = ();
    type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

    async fn load(
        &self, 
        reader: &mut dyn Reader, 
        _settings: &Self::Settings, 
        _ctx: &mut LoadContext<'_>
    ) -> Result<Self::Asset, Self::Error> {
        bevy::log::debug!("ActionSetLoader running...");
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes).await;
        let read= serde_json::from_slice(&bytes);
        let res = read.map_err(|err| {bevy::log::error!("ActionSetLoader error: {:?}", err); err.into()} );
        bevy::log::debug!("ActionSetLoader finished...");
        res
    }
}

#[derive(Resource, Default)]
struct ActionSetHandles(pub HashMap<String, Handle<ActionSet>>);


#[derive(Resource, Default)]
struct AssetLoadTimeouts(pub HashMap<String, Timer>);


#[derive(Event, Debug)]
pub struct LoadActionSetRequest {
    filename: String
}

impl LoadActionSetRequest {
    pub fn new<IS: Into<String>>(filename: IS) -> Self {
        Self {
            filename: filename.into()
        }
    }
}

#[derive(Event, Debug)]
pub struct ActionSetLoaded {
    pub filename: String,
    pub asset_handle: Handle<ActionSet>,
}

#[derive(Event, Debug)]
pub struct ActionSetLoadingTimeout {
    filename: String,
    timeout_time: f32,
}

fn load_asset(
    event: On<LoadActionSetRequest>,
    asset_server: Res<AssetServer>,
    mut handles: ResMut<ActionSetHandles>,
    mut timer: ResMut<AssetLoadTimeouts>,
) {
    let asset_path = event.event().filename.to_owned();
    println!("Reading ActionSet from {}...", &asset_path);
    let handle: Handle<ActionSet> = asset_server.load(asset_path.to_owned());
    handles.0.entry(asset_path.to_owned()).or_insert(handle);
    timer.0.insert(asset_path.to_owned(), Timer::new(Duration::from_secs(2), TimerMode::Once));
}

fn countdown(
    time: Res<Time>,
    handles: Res<ActionSetHandles>,
    assets: Res<Assets<ActionSet>>,
    mut timers: ResMut<AssetLoadTimeouts>,
    mut commands: Commands,
) {
    timers.0.iter_mut().for_each(|(key, timer)| {
        if timer.is_finished() {
            let handle = handles.0.get(key);
            let asset = handle
                .map(|handle| assets.get(handle))
                .flatten()
            ;
            
            match asset {
                Some(_loaded_data) => {
                    bevy::log::info!("Successfully loaded ActionSet from file {:?}...", key);
                    let notification = ActionSetLoaded {
                        filename: key.to_owned(),
                        asset_handle: handle.unwrap().to_owned(),
                    };
                    commands.trigger(notification);
                },
                None => {
                    let elapsed_time = timer.elapsed_secs();
                    bevy::log::warn!(
                        "Loading ActionSet data from file {:?} timed out after {:?}s!", 
                        key, elapsed_time
                    );
                    let notification = ActionSetLoadingTimeout {
                        filename: key.to_owned(),
                        timeout_time: elapsed_time,
                    };
                    commands.trigger(notification);
                },
            };
        }
        else {
            timer.tick(time.delta());
        }
    });
}


fn cleanup_timers_for_loaded_actionsets(
    event: On<ActionSetLoaded>,
    mut timers: ResMut<AssetLoadTimeouts>,
) {
    let evt = event.event();
    timers.0.remove(&evt.filename);
}


#[derive(Default)]
pub struct ActionSetAssetPlugin;

impl bevy::app::Plugin for ActionSetAssetPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app
        .add_plugins(AssetPlugin::default())
        .init_resource::<ActionSetHandles>()
        .init_asset::<ActionSet>()
        .init_asset_loader::<ActionSetLoader>()
        .init_resource::<AssetLoadTimeouts>()
        .add_observer(load_asset)
        .add_observer(cleanup_timers_for_loaded_actionsets)
        .add_systems(First, countdown)
        ;
    }
}

#[cfg(test)]
mod tests {
    use bevy::{app::ScheduleRunnerPlugin, prelude::*};
    use super::*;

    fn load_test_asset(
        mut commands: Commands,
    ) {
        let request = LoadActionSetRequest {
            filename: "unittest_assets://simpleagent.json".to_string()
        };
        commands.trigger(request);
    }

    fn succeed_on_loaded(
        trigger: On<ActionSetLoaded>,
        mut exit: MessageWriter<AppExit>,
    ) {
        let evt = trigger.event();
        bevy::log::info!("ActionSet loaded successfully from {:?} as {:?}", evt.filename, evt.asset_handle);
        exit.write(AppExit::Success);
    }

    fn fail_on_timeout(
        trigger: On<ActionSetLoadingTimeout>,
        mut exit: MessageWriter<AppExit>,
    ) {
        let evt = trigger.event();
        bevy::log::error!("ActionSet loading from {:?} timed out after {:?}s", evt.filename, evt.timeout_time);
        assert!(false);
        exit.write(AppExit::Success);
    }

    #[test]
    fn test_load() {
        let mut app = App::new();
        app
        .register_asset_source(
            "unittest_assets", 
            AssetSourceBuilder::platform_default(
                "../../assets", 
                None,
            )
        )
        .add_plugins((
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(200))),
            bevy::log::LogPlugin { 
                level: bevy::log::Level::DEBUG, 
                custom_layer: |_| None, 
                filter: "wgpu=error,bevy_render=info,bevy_ecs=info".to_string(),
                fmt_layer: |_| None,
            },
            ActionSetAssetPlugin::default(),
            // DefaultPlugins
        ))
        .add_systems(Startup, load_test_asset)
        .add_observer(succeed_on_loaded)
        .add_observer(fail_on_timeout)
        .run();
    }
}
