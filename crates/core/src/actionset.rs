
use bevy::asset::{AssetLoader, LoadContext, io::Reader, Asset};
use bevy::reflect::Reflect;
use serde::{Serialize, Deserialize};
use crate::actions::{ActionTemplate};


#[derive(Asset, Reflect, Serialize, Deserialize, Debug)]
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
        let res = read.map_err(|err| {println!("ActionSetLoader error: {:?}", err); err.into()} );
        bevy::log::debug!("ActionSetLoader finished...");
        res
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, num::NonZero, time::Duration};
    use bevy::{app::ScheduleRunnerPlugin, prelude::*};
    use super::*;

    #[derive(Resource, Default)]
    struct ActionSetHandles(HashMap<String, Handle<ActionSet>>);

    #[derive(Resource, Default)]
    struct LoadTimeout(Timer);

    fn load_asset(
        mut handles: ResMut<ActionSetHandles>,
        asset_server: Res<AssetServer>,
        mut timer: ResMut<LoadTimeout>,
    ) {
        let asset_path = "simpleagent.json";
        println!("Reading ActionSet from {}...", asset_path);
        let handle: Handle<ActionSet> = asset_server.load(asset_path);
        handles.0.entry(asset_path.to_string()).or_insert(handle);
        timer.0.set_duration(Duration::from_secs(2));
    }

    fn countdown(
        time: Res<Time>,
        mut timer: ResMut<LoadTimeout>,
    ) {
        timer.0.tick(time.delta());
    }

    fn process_asset(
        handles: Res<ActionSetHandles>,
        assets: Res<Assets<ActionSet>>,
        timer: Res<LoadTimeout>,
        mut exit: MessageWriter<AppExit>,
    ) {

        for (filename, asset_handle) in handles.0.iter() {
            let asset = assets.get(asset_handle);
            if let Some(data) = asset {
                println!("{} => {:?}", filename, data);
                exit.write(AppExit::Success);
            }
            else {
                println!("{} => <not loaded> after {}s", filename, timer.0.elapsed_secs());
                if timer.0.is_finished() {
                    exit.write(AppExit::Error(NonZero::new(2).unwrap()));
                }
            }
        };
    }

    #[test]
    fn test_load() {
        let mut app = App::new();
        app
        .add_plugins(
            MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(1000)))
            // DefaultPlugins
        )
        .add_plugins(AssetPlugin::default())
        .init_resource::<ActionSetHandles>()
        .init_asset::<ActionSet>()
        .init_asset_loader::<ActionSetLoader>()
        .init_resource::<LoadTimeout>()
        .add_systems(Startup, load_asset)
        .add_systems(Update, (countdown, process_asset))
        .run();

        let ex = app.should_exit();
        if let Some(exitevt) = ex {
            assert!(exitevt.is_success())
        }
    }
}
