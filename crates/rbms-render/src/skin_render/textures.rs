//! The textures a compiled screen holds: one per image source its document names.
//!
//! Every object that draws from an image looks its source up here while the screen is built, and
//! the screen hands the whole set back when it is released. Which sources are registered, when, and
//! for how long is decided in this module and nowhere else.

use rbms_skin::loader::LoadedSkin;

use super::SkinAssets;
use crate::{Renderer, TextureId};

/// Everything the draw list needs about the image sources that were registered, each as
/// `(document source id, texture, size in pixels)`.
pub(crate) type Source<'a> = &'a [(String, TextureId, (u32, u32))];

/// Looks an image source up by the id a document gave it.
pub(crate) fn source_of<'a>(sources: Source<'a>, id: &str) -> Option<&'a (String, TextureId, (u32, u32))> {
    sources.iter().find(|(source, _, _)| source == id)
}

/// The image sources one screen registered with a renderer.
#[derive(Debug, Default)]
pub(crate) struct SkinTextures {
    sources: Vec<(String, TextureId, (u32, u32))>,
}

impl SkinTextures {
    /// Registers every image source `skin` names, under keys in the namespace `serial` claims.
    ///
    /// An image that will not decode leaves a line in `warnings` and no source behind, so the
    /// objects that would have drawn from it are dropped one by one as they are built.
    pub(crate) fn register<R: Renderer>(r: &mut R, skin: &LoadedSkin, assets: &mut dyn SkinAssets, serial: u32, warnings: &mut Vec<String>) -> SkinTextures {
        let mut sources: Vec<(String, TextureId, (u32, u32))> = Vec::new();
        for (id, path) in &skin.sources {
            match assets.image(path) {
                Some(image) => {
                    let key = format!("rbms.skin.{serial}.source.{id}");
                    let tex = r.register_texture(&key, &image.rgba, image.width, image.height);
                    sources.push((id.clone(), tex, (image.width, image.height)));
                }
                None => warnings.push(format!("image source {id:?} could not be decoded from {}", path.display())),
            }
        }
        SkinTextures { sources }
    }

    /// The registered sources, as the object builders look them up.
    pub(crate) fn sources(&self) -> Source<'_> {
        &self.sources
    }

    /// How many textures are held.
    pub(crate) fn count(&self) -> usize {
        self.sources.len()
    }

    /// Hands every texture back to `r`, leaving nothing held.
    pub(crate) fn release<R: Renderer>(&mut self, r: &mut R) {
        for (_, tex, _) in self.sources.drain(..) {
            r.release_texture(tex);
        }
    }
}
