//! TODO(prose): renders a kittest harness at 1× and 2× on Mesa lavapipe, for review images.
//!
//! The adapter is picked in-process: of the Vulkan adapters, the one whose
//! driver is lavapipe (`llvmpipe`), even beside a real GPU.
//!
//! Two looks side by side, at real size, in both themes:
//!
//! ```no_run
//! use eggshell_ui::testing::screenshot::{self, Scale};
//! use eggshell_ui::{LayoutState, tw};
//! use egui::Color32;
//!
//! let dir = std::path::Path::new("target/shots");
//! for (scene, fill) in [("before", Color32::from_rgb(40, 90, 200)), ("after", Color32::from_rgb(200, 90, 40))] {
//!     let written = screenshot::write_shots(dir, scene, |builder, _theme| {
//!         builder
//!             .with_size(egui::vec2(120.0, 80.0))
//!             .build_ui_state(
//!                 move |ui, layout: &mut LayoutState| {
//!                     layout
//!                         .build(ui.ctx(), |b| { b.div(tw().w_px(40.0).h_px(40.0).bg(fill), |_| {}); })
//!                         .layout(ui.max_rect())
//!                         .draw(&mut (), ui);
//!                 },
//!                 LayoutState::new(eggshell_ui::theme::Shadcn::themed(eggshell_ui::theme::ShadcnStyle::Vega, eggshell_ui::theme::Palette::new())),
//!             )
//!     })?;
//!     assert_eq!(written.len(), Scale::ALL.len() * 2);
//! }
//! # Ok::<(), eggshell_ui::testing::screenshot::ScreenshotError>(())
//! ```

mod windows;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui::{Id, Theme};
use egui_kittest::wgpu::WgpuTestRenderer;
use egui_kittest::{Harness, HarnessBuilder};
use egui_wgpu::wgpu;
use image::RgbaImage;

/// TODO(prose): the pixels per point of a shot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scale {
    /// TODO(prose): one pixel per point.
    X1,
    /// TODO(prose): two pixels per point, as on a high-density display.
    X2,
}

impl Scale {
    /// TODO(prose): every scale, in the order [`write_shots`] renders them.
    pub const ALL: [Scale; 2] = [Scale::X1, Scale::X2];

    /// TODO(prose): the pixels per point egui renders this scale at.
    pub fn pixels_per_point(self) -> f32 {
        match self {
            Scale::X1 => 1.0,
            Scale::X2 => 2.0,
        }
    }

    fn factor(self) -> u8 {
        match self {
            Scale::X1 => 1,
            Scale::X2 => 2,
        }
    }
}

/// TODO(prose): why a shot could not be rendered or written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenshotError {
    /// TODO(prose): no Vulkan adapter is lavapipe; the adapters found are listed.
    NoLavapipe(String),
    /// TODO(prose): the renderer failed.
    Render(String),
    /// TODO(prose): [`render_windows`] was given a harness that embeds its
    /// windows on the page (egui's `embed_viewports`, on in kittest).
    Embedded,
    /// TODO(prose): [`render_windows`] was given a harness not on [`builder`]'s
    /// renderer.
    NotFromBuilder,
    /// TODO(prose): the image or its directory could not be written.
    Write {
        /// TODO(prose): the file or directory.
        path: PathBuf,
        /// TODO(prose): the system's reason.
        why: String,
    },
}

impl std::fmt::Display for ScreenshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoLavapipe(found) => write!(
                f,
                "no Mesa lavapipe Vulkan adapter (install mesa-vulkan-drivers); found: {found}"
            ),
            Self::Render(why) => write!(f, "rendering the screenshot failed: {why}"),
            Self::Embedded => write!(
                f,
                "the harness embeds its pop-outs on the page: call `ctx.set_embed_viewports(false)` first"
            ),
            Self::NotFromBuilder => write!(
                f,
                "the harness was not built from `screenshot::builder`, whose renderer paints pop-outs"
            ),
            Self::Write { path, why } => write!(f, "writing {} failed: {why}", path.display()),
        }
    }
}

impl std::error::Error for ScreenshotError {}

/// TODO(prose): a kittest renderer on lavapipe, or why there is none.
pub fn lavapipe() -> Result<WgpuTestRenderer, ScreenshotError> {
    render_state().map(WgpuTestRenderer::from_render_state)
}

fn render_state() -> Result<egui_wgpu::RenderState, ScreenshotError> {
    let mut setup = egui_wgpu::WgpuSetupCreateNew::without_display_handle();
    setup.instance_descriptor.backends = wgpu::Backends::VULKAN;
    setup.native_adapter_selector = Some(Arc::new(|adapters, _surface| {
        adapters
            .iter()
            .find(|adapter| is_lavapipe(&adapter.get_info()))
            .cloned()
            .ok_or_else(|| describe(adapters))
    }));
    let setup = egui_wgpu::WgpuSetup::CreateNew(setup);
    let instance = pollster::block_on(setup.new_instance());
    let config = egui_wgpu::WgpuConfiguration {
        wgpu_setup: setup,
        ..Default::default()
    };
    pollster::block_on(egui_wgpu::RenderState::create(
        &config,
        &instance,
        None,
        egui_wgpu::RendererOptions::PREDICTABLE,
    ))
    .map_err(|error| match error {
        egui_wgpu::WgpuError::CustomNativeAdapterSelectionError(found) => {
            ScreenshotError::NoLavapipe(found)
        }
        other => ScreenshotError::Render(other.to_string()),
    })
}

/// TODO(prose): whether `info` is the adapter of Mesa lavapipe, the renderer shots and goldens are drawn by.
pub fn is_lavapipe(info: &wgpu::AdapterInfo) -> bool {
    info.backend == wgpu::Backend::Vulkan
        && info.device_type == wgpu::DeviceType::Cpu
        && (info.driver == "llvmpipe" || info.name.starts_with("llvmpipe"))
}

fn describe(adapters: &[wgpu::Adapter]) -> String {
    if adapters.is_empty() {
        return "no Vulkan adapter".to_owned();
    }
    adapters
        .iter()
        .map(|adapter| {
            let info = adapter.get_info();
            format!("{} ({}, {:?})", info.name, info.driver, info.device_type)
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// TODO(prose): a harness builder on lavapipe at `scale` in `theme`.
///
/// Its renderer also paints the harness's pop-outs for [`render_windows`]:
/// building it shows the native windows of every harness on this thread
/// through egui's immediate-viewport renderer.
pub fn builder<State>(
    scale: Scale,
    theme: Theme,
) -> Result<HarnessBuilder<State>, ScreenshotError> {
    let renderer = windows::Lavapipe::new(render_state()?);
    windows::install();
    Ok(Harness::builder()
        .renderer(renderer)
        .with_pixels_per_point(scale.pixels_per_point())
        .with_theme(theme))
}

/// TODO(prose): the harness's last frame as an image.
pub fn render<State>(harness: &mut Harness<'_, State>) -> Result<RgbaImage, ScreenshotError> {
    harness.render().map_err(ScreenshotError::Render)
}

/// TODO(prose): each native pop-out window of `harness`, by its pop-out's id,
/// as the next step draws it, at its size and the harness's scale.
///
/// The harness comes from [`builder`] and shows its pop-outs as native
/// windows: `harness.ctx.set_embed_viewports(false)`. This runs one step.
pub fn render_windows<State>(
    harness: &mut Harness<'_, State>,
) -> Result<Vec<(Id, RgbaImage)>, ScreenshotError> {
    if harness.ctx.embed_viewports() {
        return Err(ScreenshotError::Embedded);
    }
    windows::capture(|| harness.step())
        .ok_or(ScreenshotError::NotFromBuilder)?
        .into_iter()
        .map(|(id, shot)| {
            shot.map(|image| (id, image))
                .map_err(ScreenshotError::Render)
        })
        .collect()
}

/// TODO(prose): the file name of a shot, `<scene>-<theme>@<scale>x.png`.
pub fn file_name(scene: &str, theme: Theme, scale: Scale) -> String {
    let theme = match theme {
        Theme::Light => "light",
        Theme::Dark => "dark",
    };
    format!("{scene}-{theme}@{}x.png", scale.factor())
}

/// TODO(prose): renders `scene` in light and dark at 1× and 2× into `dir`, and returns the files written.
///
/// `build` makes the harness from a builder already on lavapipe at the shot's
/// scale and theme, and leaves it in the state to show.
pub fn write_shots<'a, State>(
    dir: &Path,
    scene: &str,
    mut build: impl FnMut(HarnessBuilder<State>, Theme) -> Harness<'a, State>,
) -> Result<Vec<PathBuf>, ScreenshotError> {
    std::fs::create_dir_all(dir).map_err(|error| ScreenshotError::Write {
        path: dir.to_owned(),
        why: error.to_string(),
    })?;
    let mut written = Vec::new();
    for theme in [Theme::Light, Theme::Dark] {
        for scale in Scale::ALL {
            let mut harness = build(builder(scale, theme)?, theme);
            let path = dir.join(file_name(scene, theme, scale));
            render(&mut harness)?
                .save(&path)
                .map_err(|error| ScreenshotError::Write {
                    path: path.clone(),
                    why: error.to_string(),
                })?;
            written.push(path);
        }
    }
    Ok(written)
}
