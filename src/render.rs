use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::Query;
use bevy_ecs::{
    system::{Commands, Res},
    world::World,
};
use bevy_ecs::resource::Resource;
use bevy_render::render_graph::RenderLabel;
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraphContext},
    renderer::RenderContext,
    view::ExtractedWindows,
    Extract,
};
use bevy_window::Window;
use iced_core::Size;
use iced_wgpu::wgpu::TextureFormat;
use iced_wgpu::graphics::Viewport;

use crate::{DidDraw, IcedResource, IcedSettings};

#[derive(Clone, Hash, Debug, Eq, PartialEq, RenderLabel)]
pub struct IcedPass;

#[cfg(target_arch = "wasm32")]
pub const TEXTURE_FMT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
#[cfg(not(target_arch = "wasm32"))]
pub const TEXTURE_FMT: TextureFormat = TextureFormat::Bgra8UnormSrgb;

#[derive(Resource, Deref, DerefMut, Clone)]
pub struct ViewportResource(pub Viewport);

pub fn update_viewport(
    windows: Query<&Window>,
    iced_settings: Res<IcedSettings>,
    mut commands: Commands,
) {
    let Ok(window) = windows.single() else { return };
    let scale_factor = iced_settings
        .scale_factor
        .unwrap_or_else(|| window.scale_factor().into());
    let viewport = Viewport::with_physical_size(
        Size::new(window.physical_width(), window.physical_height()),
        scale_factor as f32,
    );
    commands.insert_resource(ViewportResource(viewport));
}

// Same as DidDraw, but as a regular bool instead of an atomic.
#[derive(Resource, Deref, DerefMut)]
struct DidDrawBasic(bool);

pub fn extract_iced_data(
    mut commands: Commands,
    viewport: Extract<Res<ViewportResource>>,
    did_draw: Extract<Res<DidDraw>>,
) {
    commands.insert_resource(ViewportResource(viewport.0.clone()));
    commands.insert_resource(DidDrawBasic(
        did_draw.0.swap(false, std::sync::atomic::Ordering::Relaxed),
    ));
}

pub struct IcedNode;

impl IcedNode {
    pub fn new() -> Self {
        Self
    }
}

impl Node for IcedNode {
    fn update(&mut self, _world: &mut World) {}

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let Some(extracted_window) = world
            .get_resource::<ExtractedWindows>()
            .unwrap()
            .windows
            .values()
            .next()
        else {
            return Ok(());
        };

        let viewport = world.resource::<ViewportResource>();

        if !world.get_resource::<DidDrawBasic>().is_some_and(|x| x.0) {
            return Ok(());
        }
        let view = extracted_window.swap_chain_texture_view.as_ref().unwrap();

        let mut iced_resource = world.get_non_send_resource::<IcedResource>().unwrap().lock().unwrap();
        iced_resource.renderer.present(
            None,
            TEXTURE_FMT,
            &**view,
            viewport,
        );

        Ok(())
    }
}
