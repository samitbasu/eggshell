//! Native pop-out windows on lavapipe: the immediate-viewport renderer that
//! runs each window's pass beside the harness's, and the harness renderer
//! that paints the windows' passes when a capture asks for them.
//!
//! egui calls the immediate-viewport renderer inside the harness's step, and
//! kittest hands the step's textures to the harness renderer right after it,
//! so a window's pass waits on this thread for the end of its step. The
//! window's textures are applied there first: egui takes the texture changes
//! of a frame at the end of the first pass to finish, which is the window's.

use core::time::Duration;
use std::cell::RefCell;

use egui::epaint::ClippedPrimitive;
use egui::{Context, Id, ImmediateViewport, Pos2, RawInput, Rect, TexturesDelta, ViewportInfo};
use egui_kittest::TestRenderer;
use egui_kittest::wgpu::WgpuTestRenderer;
use egui_wgpu::{RenderState, ScreenDescriptor, wgpu};
use image::RgbaImage;

/// How long a capture waits for lavapipe, as kittest's own renders do.
const WAIT: Duration = Duration::from_secs(10);

/// A window's pass, waiting for the end of its step.
struct Pass {
    id: Id,
    textures: TexturesDelta,
    painted: Option<Painted>,
}

/// What a captured window's pass painted, tessellated.
struct Painted {
    primitives: Vec<ClippedPrimitive>,
    screen: ScreenDescriptor,
}

/// The windows' passes of the step of a harness: its context and page pass.
#[derive(Default)]
struct Step {
    of: Option<(Context, u64)>,
    passes: Vec<Pass>,
}

/// What became of the windows of a capture's step.
enum Capture {
    Off,
    Asked,
    Done(Vec<(Id, Result<RgbaImage, String>)>),
}

thread_local! {
    static STEP: RefCell<Step> = RefCell::default();
    static CAPTURE: RefCell<Capture> = const { RefCell::new(Capture::Off) };
}

/// Shows the native windows of every harness on this thread from now on.
pub(super) fn install() {
    Context::set_immediate_viewport_renderer(show);
}

/// Runs `step` asking for the windows it shows to be painted, and returns
/// them, or `None` when no [`Lavapipe`] renderer took the step.
pub(super) fn capture(step: impl FnOnce()) -> Option<Vec<(Id, Result<RgbaImage, String>)>> {
    CAPTURE.set(Capture::Asked);
    step();
    match CAPTURE.replace(Capture::Off) {
        Capture::Done(shots) => Some(shots),
        Capture::Off | Capture::Asked => None,
    }
}

fn asked() -> bool {
    CAPTURE.with_borrow(|capture| matches!(capture, Capture::Asked))
}

/// Runs the pass of the native window `viewport`, at its builder's inner
/// size and the page's scale.
fn show(ctx: &Context, viewport: ImmediateViewport<'_>) {
    let id = viewport.ids.this;
    let page = (ctx.clone(), ctx.cumulative_pass_nr());
    let size = viewport
        .builder
        .inner_size
        .unwrap_or_else(|| ctx.content_rect().size());
    let (time, predicted_dt) = ctx.input(|input| (input.time, input.predicted_dt));
    let info = ViewportInfo {
        native_pixels_per_point: ctx.native_pixels_per_point(),
        ..ViewportInfo::default()
    };
    let input = RawInput {
        viewport_id: id,
        viewports: std::iter::once((id, info)).collect(),
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        time: Some(time),
        predicted_dt,
        ..RawInput::default()
    };
    let mut pass_ui = viewport.viewport_ui_cb;
    let mut output = ctx.run_ui(input, |ui| pass_ui(ui));
    let painted = asked().then(|| {
        let pixels_per_point = output.pixels_per_point;
        let pixels = size * pixels_per_point;
        Painted {
            primitives: ctx.tessellate(std::mem::take(&mut output.shapes), pixels_per_point),
            screen: ScreenDescriptor {
                size_in_pixels: [pixels.x.round() as u32, pixels.y.round() as u32],
                pixels_per_point,
            },
        }
    });
    STEP.with_borrow_mut(|step| {
        if step.of.as_ref() != Some(&page) {
            *step = Step {
                of: Some(page),
                passes: Vec::new(),
            };
        }
        step.passes.push(Pass {
            id: id.0,
            textures: std::mem::take(&mut output.textures_delta),
            painted,
        });
    });
}

/// kittest's lavapipe renderer, which also takes the textures of the
/// windows' passes of each step, and paints them for a capture.
pub(super) struct Lavapipe {
    page: WgpuTestRenderer,
    state: RenderState,
}

impl Lavapipe {
    pub(super) fn new(state: RenderState) -> Self {
        Self {
            page: WgpuTestRenderer::from_render_state(state.clone()),
            state,
        }
    }
}

impl TestRenderer for Lavapipe {
    fn handle_delta(&mut self, delta: &mut TexturesDelta) {
        let passes = STEP.take().passes;
        let mut shots = Vec::new();
        for mut pass in passes {
            let free = std::mem::take(&mut pass.textures.free);
            self.page.handle_delta(&mut pass.textures);
            if let Some(painted) = pass.painted {
                shots.push((pass.id, paint(&self.state, &painted)));
            }
            let mut freed = TexturesDelta::default();
            freed.free = free;
            self.page.handle_delta(&mut freed);
        }
        CAPTURE.with_borrow_mut(|capture| {
            if matches!(capture, Capture::Asked) {
                *capture = Capture::Done(shots);
            }
        });
        self.page.handle_delta(delta);
    }

    fn render(
        &mut self,
        ctx: &egui::Context,
        output: &egui::FullOutput,
    ) -> Result<RgbaImage, String> {
        self.page.render(ctx, output)
    }
}

/// Paints a window's pass into an image of its size, cleared to
/// transparent as eframe clears a pop-out's.
fn paint(state: &RenderState, painted: &Painted) -> Result<RgbaImage, String> {
    let [width, height] = painted.screen.size_in_pixels;
    let largest = state.device.limits().max_texture_dimension_2d;
    if width == 0 || height == 0 || width > largest || height > largest {
        return Err(format!(
            "a window of {width} x {height} pixels can't be painted (at most {largest})"
        ));
    }
    let (device, queue) = (&state.device, &state.queue);
    let extent = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("eggshell pop-out"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: state.target_format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    let mut renderer = state.renderer.write();
    let buffers = renderer.update_buffers(
        device,
        queue,
        &mut encoder,
        &painted.primitives,
        &painted.screen,
    );
    {
        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("eggshell pop-out"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            })
            .forget_lifetime();
        renderer.render(&mut pass, &painted.primitives, &painted.screen);
    }
    drop(renderer);
    let row = 4 * width;
    let padded = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let read = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("eggshell pop-out pixels"),
        size: u64::from(padded) * u64::from(height),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &read,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: None,
            },
        },
        extent,
    );
    let submitted = queue.submit(buffers.into_iter().chain(std::iter::once(encoder.finish())));
    let (sender, receiver) = std::sync::mpsc::channel();
    read.slice(..)
        .map_async(wgpu::MapMode::Read, move |mapped| drop(sender.send(mapped)));
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submitted),
            timeout: Some(WAIT),
        })
        .map_err(|error| error.to_string())?;
    receiver
        .recv()
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())?;
    let mapped = read
        .slice(..)
        .get_mapped_range()
        .map_err(|error| error.to_string())?;
    let pixels = mapped
        .chunks_exact(padded as usize)
        .flat_map(|line| line.iter().take(row as usize))
        .copied()
        .collect();
    RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| "the pixels read back short".to_owned())
}
