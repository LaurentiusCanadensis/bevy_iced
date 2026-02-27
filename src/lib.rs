//! # Use Iced UI programs in your Bevy application
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_iced::iced::widget::text;
//! use bevy_iced::{IcedContext, IcedPlugin};
//!
//! #[derive(Event)]
//! pub enum UiMessage {}
//!
//! pub fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(IcedPlugin::default())
//!         .add_event::<UiMessage>()
//!         .add_systems(Update, ui_system)
//!         .run();
//! }
//!
//! fn ui_system(time: Res<Time>, mut ctx: IcedContext<UiMessage>) {
//!     ctx.display(text(format!(
//!         "Hello Iced! Running for {:.2} seconds.",
//!         time.elapsed_secs()
//!     )));
//! }
//! ```

#![deny(missing_docs)]

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::render::{extract_iced_data, IcedNode, ViewportResource};

use bevy_app::{App, Plugin, Update};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::{Query, With};
use bevy_ecs::message::MessageWriter;
use bevy_ecs::resource::Resource;
use bevy_ecs::system::{NonSendMut, Res, ResMut, SystemParam};
use bevy_input::touch::Touches;
use bevy_render::render_graph::RenderGraph;
use bevy_render::renderer::{RenderAdapter, RenderDevice, RenderQueue};
use bevy_render::{ExtractSchedule, RenderApp};
use bevy_window::{PrimaryWindow, Window};
use iced_core::mouse::Cursor;
use iced_runtime::user_interface::UserInterface;
use iced_wgpu::graphics::Viewport;

/// Basic re-exports for all Iced-related stuff.
///
/// This module attempts to emulate the `iced` package's API
/// as much as possible.
pub mod iced;

mod conversions;
mod render;
mod systems;
mod utils;

use systems::IcedEventQueue;

/// The default renderer.
pub type Renderer = iced_wgpu::Renderer;

/// The main feature of `bevy_iced`.
/// Add this to your [`App`] by calling `app.add_plugin(bevy_iced::IcedPlugin::default())`.
#[derive(Default)]
pub struct IcedPlugin;

impl Plugin for IcedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (systems::process_input, render::update_viewport))
            .insert_resource(DidDraw::default())
            .insert_resource(IcedSettings::default())
            .insert_non_send_resource(IcedCache::default())
            .insert_resource(IcedEventQueue::default());
    }

    fn finish(&self, app: &mut App) {
        let default_viewport = Viewport::with_physical_size(iced_core::Size::new(1600, 900), 1.0);
        let default_viewport = ViewportResource(default_viewport);
        let iced_resource: IcedResource = IcedProps::new(app).into();

        app.insert_resource(default_viewport.clone())
            .insert_non_send_resource(iced_resource.clone());

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .insert_resource(default_viewport)
            .add_systems(ExtractSchedule, extract_iced_data);
        render_app.world_mut().insert_non_send_resource(iced_resource);
        let mut graph = render_app.world_mut().get_resource_mut::<RenderGraph>().unwrap();
        setup_pipeline(&mut *graph);
    }
}

pub(crate) struct IcedProps {
    renderer: Renderer,
    clipboard: iced_core::clipboard::Null,
}

impl IcedProps {
    fn new(app: &App) -> Self {
        use std::ops::Deref;

        let render_world = app.sub_app(RenderApp).world();
        let render_device = render_world.get_resource::<RenderDevice>().unwrap();
        let render_queue = render_world.get_resource::<RenderQueue>().unwrap();
        let render_adapter = render_world.get_resource::<RenderAdapter>().unwrap();

        // Clone the wgpu types out of Bevy's wrappers — Engine::new takes owned Device/Queue.
        // In wgpu 27 Device and Queue are cheaply Clone (internal Arc).
        let device: iced_wgpu::wgpu::Device = render_device.wgpu_device().clone();
        let queue: iced_wgpu::wgpu::Queue = render_queue.0.deref().deref().clone();
        let adapter_ref: &iced_wgpu::wgpu::Adapter = render_adapter.0.deref().deref();

        let engine = iced_wgpu::Engine::new(
            adapter_ref,
            device,
            queue,
            render::TEXTURE_FMT,
            None,
            iced_wgpu::graphics::Shell::headless(),
        );

        let renderer = iced_wgpu::Renderer::new(
            engine,
            iced_core::Font::DEFAULT,
            iced_core::Pixels(16.0),
        );

        Self {
            renderer,
            clipboard: iced_core::clipboard::Null,
        }
    }
}

// SAFETY: IcedProps is only accessed from the main thread via NonSend resource,
// or from the render thread behind a Mutex. The Rc's inside Renderer are never
// shared across threads — access is serialized by the Mutex.
unsafe impl Send for IcedProps {}
unsafe impl Sync for IcedProps {}

#[derive(Clone)]
pub(crate) struct IcedResource(Arc<Mutex<IcedProps>>);

impl IcedResource {
    fn lock(&self) -> std::sync::LockResult<std::sync::MutexGuard<'_, IcedProps>> {
        self.0.lock()
    }
}

impl From<IcedProps> for IcedResource {
    fn from(value: IcedProps) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }
}

fn setup_pipeline(graph: &mut RenderGraph) {
    graph.add_node(render::IcedPass, IcedNode::new());

    graph.add_node_edge(bevy_render::graph::CameraDriverLabel, render::IcedPass);
}

#[derive(Default)]
struct IcedCache {
    cache: HashMap<TypeId, Option<iced_runtime::user_interface::Cache>>,
}

impl IcedCache {
    fn get<M: Any>(&mut self) -> &mut Option<iced_runtime::user_interface::Cache> {
        let id = TypeId::of::<M>();
        if !self.cache.contains_key(&id) {
            self.cache.insert(id, Some(Default::default()));
        }
        self.cache.get_mut(&id).unwrap()
    }
}

/// Settings used to independently customize Iced rendering.
#[derive(Clone, Resource)]
pub struct IcedSettings {
    /// The scale factor to use for rendering Iced elements.
    /// Setting this to `None` defaults to using the `Window`s scale factor.
    pub scale_factor: Option<f64>,
    /// The theme to use for rendering Iced elements.
    pub theme: iced_core::Theme,
    /// The style to use for rendering Iced elements.
    pub style: iced_core::renderer::Style,
}

impl IcedSettings {
    /// Set the `scale_factor` used to render Iced elements.
    pub fn set_scale_factor(&mut self, factor: impl Into<Option<f64>>) {
        self.scale_factor = factor.into();
    }
}

impl Default for IcedSettings {
    fn default() -> Self {
        Self {
            scale_factor: None,
            theme: iced_core::Theme::Dark,
            style: iced_core::renderer::Style {
                text_color: iced_core::Color::WHITE,
            },
        }
    }
}

// An atomic flag for updating the draw state.
#[derive(Resource, Deref, DerefMut, Default)]
pub(crate) struct DidDraw(std::sync::atomic::AtomicBool);

/// The context for interacting with Iced. Add this as a parameter to your system.
/// ```ignore
/// fn ui_system(..., mut ctx: IcedContext<UiMessage>) {
///     let element = ...; // Build your element
///     ctx.display(element);
/// }
/// ```
///
/// `IcedContext<T>` requires a message type registered in the [`App`].
#[derive(SystemParam)]
pub struct IcedContext<'w, 's, Message: bevy_ecs::message::Message> {
    viewport: Res<'w, ViewportResource>,
    props: NonSendMut<'w, IcedResource>,
    settings: Res<'w, IcedSettings>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    events: ResMut<'w, IcedEventQueue>,
    cache_map: NonSendMut<'w, IcedCache>,
    messages: MessageWriter<'w, Message>,
    did_draw: ResMut<'w, DidDraw>,
    touches: Res<'w, Touches>,
}

impl<'w, 's, M: bevy_ecs::message::Message> IcedContext<'w, 's, M> {
    /// Display an [`Element`] to the screen.
    pub fn display<'a>(
        &'a mut self,
        element: impl Into<iced_core::Element<'a, M, iced_core::Theme, Renderer>>,
    ) {
        let IcedProps {
            ref mut renderer,
            ref mut clipboard,
            ..
        } = &mut *self.props.lock().unwrap();
        let bounds = self.viewport.logical_size();

        let element = element.into();

        let cursor = {
            let window = self.windows.single().unwrap();
            match window.cursor_position() {
                Some(position) => {
                    Cursor::Available(utils::process_cursor_position(position, bounds, window))
                }
                None => utils::process_touch_input(self)
                    .map(Cursor::Available)
                    .unwrap_or(Cursor::Unavailable),
            }
        };

        let mut messages = Vec::<M>::new();
        let cache_entry = self.cache_map.get::<M>();
        let cache = cache_entry.take().unwrap();
        let mut ui = UserInterface::build(element, bounds, cache, renderer);
        let (_, _event_statuses) = ui.update(
            self.events.0.as_slice(),
            cursor,
            renderer,
            clipboard,
            &mut messages,
        );

        messages.into_iter().for_each(|msg| {
            self.messages.write(msg);
        });

        ui.draw(renderer, &self.settings.theme, &self.settings.style, cursor);

        self.events.0.clear();
        *cache_entry = Some(ui.into_cache());
        self.did_draw.0
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}
