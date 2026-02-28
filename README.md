# `bevy_iced`: use [Iced](https://github.com/iced-rs/iced) UI programs in your [Bevy](https://github.com/bevyengine/bevy/) application

Fork of [tasgon/bevy_iced](https://github.com/tasgon/bevy_iced) with Bevy 0.18 + Iced 0.14 support. This was ported for use in a personal project and comes with no guarantees of correctness or stability.

[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](./LICENSE)

## Changes from upstream

- Ported to **Bevy 0.18** and **Iced 0.14** (wgpu 27)
- `IcedResource` is now a regular `Resource` (was `NonSend`) for thread-safe access via `Arc<Mutex>`
- Messages use Bevy 0.18's `Message` trait instead of `Event`
- Local `iced_wgpu` fork with `render_to_encoder()` for direct rendering into Bevy's command encoder
- `IcedClearColor` resource for controlling framebuffer clear in the Iced render pass

### Important: cameras must stay active

Bevy skips swap chain presentation when all cameras are deactivated. IcedNode runs after `CameraDriverLabel` in the render graph, so it overwrites camera output. Do **not** deactivate all cameras when using bevy_iced.

## Example

```rust
use bevy::prelude::*;
use bevy_iced::iced::widget::text;
use bevy_iced::{IcedContext, IcedPlugin};

#[derive(bevy::ecs::message::Message, Clone, Debug)]
pub enum UiMessage {}

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(IcedPlugin::default())
        .add_message::<UiMessage>()
        .add_systems(Update, ui_system)
        .run();
}

fn ui_system(time: Res<Time>, mut ctx: IcedContext<UiMessage>) {
    ctx.display(text(format!(
        "Hello Iced! Running for {:.2} seconds.",
        time.elapsed_secs()
    )));
}
```

See the [examples](https://github.com/tasgon/bevy_iced/tree/master/examples) for more details.

## Compatibility

|Bevy Version  |Iced Version  |Crate Version           |
|--------------|--------------|------------------------|
|`0.18`        |`0.14`        |this fork (`master`)    |
|`0.13`        |`0.13`        |`0.5` (upstream)        |
|`0.11`        |              |`0.4` (upstream)        |
|`0.10`        |              |`0.3` (upstream)        |
|`0.9`         |              |`0.2` (upstream)        |
|`0.7`         |              |`0.1` (upstream)        |

## License

MIT/Apache 2.0, same as upstream.

## Credits

- [tasgon](https://github.com/tasgon) for the original `bevy_iced` crate
- [`bevy_egui`](https://github.com/mvlabat/bevy_egui) for giving a useful starting point
- [Joonas Satka](https://github.com/jsatka) for helping port to Bevy 0.11
- [Tomas Zemanovic](https://github.com/tzemanovic) and [Julia Naomi](https://github.com/naomijub) for helping port to Bevy 0.13