//! Image and plot texture loading and rasterization cache for egui.
//!
//! Handles rasterizing SVG files with `resvg` and loading PNG artifacts into
//! `egui::TextureHandle`s cached in memory.

use egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{error, warn};

/// In-memory cache for loaded plot and graphics textures.
pub struct TextureCache {
    textures: HashMap<PathBuf, TextureHandle>,
}

impl Default for TextureCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TextureCache {
    /// Creates an empty texture cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    /// Gets or loads a texture from a filesystem path (SVG or PNG).
    pub fn get_or_load(&mut self, ctx: &Context, path: impl AsRef<Path>) -> Option<&TextureHandle> {
        let path_buf = path.as_ref().to_path_buf();
        if !self.textures.contains_key(&path_buf) {
            let texture = Self::load_texture(ctx, &path_buf)?;
            self.textures.insert(path_buf.clone(), texture);
        }
        self.textures.get(&path_buf)
    }

    fn load_texture(ctx: &Context, path: &Path) -> Option<TextureHandle> {
        if !path.exists() {
            warn!("texture file does not exist: {}", path.display());
            return None;
        }

        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        let image = match extension.as_str() {
            "svg" => Self::load_svg(path)?,
            "png" | "jpg" | "jpeg" => Self::load_raster(path)?,
            _ => {
                warn!("unsupported image format: {}", path.display());
                return None;
            }
        };

        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("image");
        Some(ctx.load_texture(name, image, TextureOptions::LINEAR))
    }

    fn load_svg(path: &Path) -> Option<ColorImage> {
        let svg_data = fs::read(path).ok()?;
        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_data(&svg_data, &opt)
            .map_err(|e| error!("failed to parse SVG {}: {}", path.display(), e))
            .ok()?;

        let pixmap_size = tree.size().to_int_size();
        let width = pixmap_size.width().clamp(1, 2048);
        let height = pixmap_size.height().clamp(1, 2048);

        let mut pixmap = tiny_skia::Pixmap::new(width, height)?;
        resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());

        let pixels = pixmap
            .data()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|chunk| Color32::from_rgba_premultiplied(chunk[0], chunk[1], chunk[2], chunk[3]))
            .collect::<Vec<_>>();

        Some(ColorImage {
            size: [width as usize, height as usize],
            pixels,
        })
    }

    fn load_raster(path: &Path) -> Option<ColorImage> {
        let data = fs::read(path).ok()?;
        let img = image::load_from_memory(&data)
            .map_err(|e| error!("failed to load image {}: {}", path.display(), e))
            .ok()?
            .into_rgba8();

        let (width, height) = img.dimensions();
        let pixels = img
            .as_raw()
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();

        Some(ColorImage {
            size: [width as usize, height as usize],
            pixels,
        })
    }
}
