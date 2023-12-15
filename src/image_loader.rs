use bevy::{
    asset::{AssetPath, LoadState},
    prelude::{
        AssetEvent, AssetServer, Assets, CoreStage, EventReader, Handle, Image, Plugin, Res,
        ResMut, Resource,
    },
    render::texture::ImageSampler,
    utils::HashMap,
};

pub struct ImageLoaderPlugin;

impl Plugin for ImageLoaderPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<ImageLoader>()
            .add_system_to_stage(CoreStage::PreUpdate, image_loader);
    }
}

#[derive(Default, Resource)]
pub struct ImageLoader {
    paths: HashMap<AssetPath<'static>, Box<dyn 'static + Send + Sync + FnMut(&mut Image)>>,
    images: HashMap<
        Handle<Image>,
        (
            LoadState,
            Box<dyn 'static + Send + Sync + FnMut(&mut Image)>,
        ),
    >,
}

impl ImageLoader {
    pub fn load_with<P, F>(
        &mut self,
        asset_server: &AssetServer,
        path: P,
        f: F,
    ) -> Handle<Image>
    where
        P: Into<AssetPath<'static>>,
        F: 'static + Send + Sync + FnMut(&mut Image),
    {
        let handle = asset_server.load::<Image, _>(path);
        self.images
            .try_insert(handle.clone(), (LoadState::NotLoaded, Box::new(f)))
            .ok();
        handle
    }

    pub fn load<P: Into<AssetPath<'static>>>(
        &mut self,
        asset_server: &AssetServer,
        path: P,
    ) -> Handle<Image> {
        self.load_with(asset_server, path, |_| ())
    }

    pub fn load_with_sampler<P: Into<AssetPath<'static>>>(
        &mut self,
        asset_server: &AssetServer,
        path: P,
        sampler: ImageSampler,
    ) -> Handle<Image> {
        self.load_with(asset_server, path, move |image: &mut Image| {
            image.sampler_descriptor = sampler.clone()
        })
    }

    pub fn is_loaded(&self, handle: &Handle<Image>) -> bool {
        let Some((load_state, _)) = self.images.get(handle) else {
            return false
        };

        *load_state == LoadState::Loaded
    }
}

pub fn image_loader(
    mut image_loader: ResMut<ImageLoader>,
    asset_server: Res<AssetServer>,
    mut image_events: EventReader<AssetEvent<Image>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (path, f) in image_loader.paths.drain().collect::<Vec<_>>() {
        let handle = asset_server.load::<Image, _>(path);
        image_loader
            .images
            .try_insert(handle, (LoadState::NotLoaded, f))
            .ok();
    }

    for event in image_events.iter() {
        let handle = match event {
            AssetEvent::Created { handle } => handle,
            AssetEvent::Modified { handle } => handle,
            AssetEvent::Removed { handle } => handle,
        };

        let Some((load_state, f)) = image_loader.images.get_mut(handle) else {
        continue
    };

        *load_state = asset_server.get_load_state(handle);

        match event {
            AssetEvent::Created { .. } => {
                let image = images.get_mut(handle).expect("Invalid image");
                f(image);
            }
            _ => (),
        }
    }
}

