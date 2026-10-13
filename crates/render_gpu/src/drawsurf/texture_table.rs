use std::num::NonZeroU32;

use super::gpu_contract::{
    TEXTURE_TABLE_2D_CAPACITY, TEXTURE_TABLE_3D_CAPACITY, TEXTURE_TABLE_CUBE_CAPACITY,
    TEXTURE_TABLE_SAMPLER_CAPACITY,
};
use super::gpu_resources::{DecodedSampler, UploadedTextureBind, UploadedTextureIdentity};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{
    BindGroup, BindGroupEntry, BindGroupLayoutDescriptor, BindingResource, Extent3d, Sampler,
    SamplerDescriptor, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor, TextureViewDimension, TextureViewId, WgpuSampler,
    WgpuTextureView,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::settings::WgpuFeatures;
use d3d9_sm3::{TEXTURE_TABLE_BINDING_2D, TEXTURE_TABLE_BINDING_3D, TEXTURE_TABLE_BINDING_CUBE};
use render_material::{MaterialGenerationId, SamplerTextureDimension};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SamplerKey {
    sampler_state: u8,
    packed_word: u32,
    anisotropy_clamp: u16,
    uses_mipmaps: bool,
}

impl From<&DecodedSampler> for SamplerKey {
    fn from(sampler: &DecodedSampler) -> Self {
        Self {
            sampler_state: sampler.sampler_state,
            packed_word: sampler.packed_word(),
            anisotropy_clamp: sampler.anisotropy_clamp(),
            uses_mipmaps: sampler.uses_mipmaps(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TextureTableRefusal {
    TexturesFull {
        dimension: SamplerTextureDimension,
        capacity: u32,
    },
    SamplersFull {
        capacity: u32,
    },
}

pub(super) struct TextureTableBinds {
    pub scene: BindGroup,
    pub sun_caster: BindGroup,
}

struct Placeholders {
    views: [TextureView; 3],
    sampler: Sampler,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub(super) struct ShadowTextureTable(pub ExactTextureTable);

#[derive(Resource, Default, Deref, DerefMut)]
pub(super) struct SceneTextureTables(pub [ExactTextureTable; 4]);

pub(super) fn scene_table_index(after_scene_resolve: bool, srgb_write: bool) -> usize {
    usize::from(after_scene_resolve) * 2 + usize::from(srgb_write)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct TableEpoch {
    pub generation: MaterialGenerationId,

    pub replaced_revision: u64,
}

#[derive(Resource, Default)]
pub(super) struct ExactTextureTable {
    epoch: TableEpoch,
    views: [Vec<TextureView>; 3],
    view_index: HashMap<TextureViewId, u16>,

    sun_shadow_indices: Vec<u16>,
    /// Slots whose view is an `R32Float` render target (sun shadow map, spot
    /// shadow map, float depth). A GPU without `FLOAT32_FILTERABLE` (A15 and
    /// older) cannot bind these to a filterable layout slot: wgpu rejects the
    /// whole bind group and the app exits. Each entry is (slot, what it is).
    r32_float_slots: Vec<(u16, &'static str)>,
    samplers: Vec<Sampler>,
    sampler_index: HashMap<SamplerKey, u16>,
    placeholders: Option<Placeholders>,
    /// 1x1 white `R8Unorm`, bound where an `R32Float` image cannot be: an
    /// effect that reads it sees "fully lit / far", the same value the shadow
    /// map is cleared to. Needs the queue to write its one texel.
    white_2d: Option<TextureView>,
    queue: Option<RenderQueue>,
    binds: Option<TextureTableBinds>,

    pub rebuild_n: u32,
}

/// Which render-target images are `R32Float` (not filterable without the
/// `FLOAT32_FILTERABLE` feature). `None` for every other texture.
fn r32_float_identity(identity: UploadedTextureIdentity) -> Option<&'static str> {
    match identity {
        UploadedTextureIdentity::Code(c) if c == super::CODE_TEXTURE_SHADOWMAP_SUN => {
            Some("sun shadow map")
        }
        UploadedTextureIdentity::Code(c) if c == super::CODE_TEXTURE_SHADOWMAP_SPOT => {
            Some("spot shadow map")
        }
        UploadedTextureIdentity::Code(c) if c == super::CODE_TEXTURE_FLOATZ => Some("float depth"),
        UploadedTextureIdentity::SpotShadowRt(_) => Some("spot shadow map"),
        _ => None,
    }
}

static R32_MASK_LOGGED: AtomicBool = AtomicBool::new(false);

/// One line in the boot log naming what was replaced and why.
fn note_r32_masked(slots: &[(u16, &'static str)]) {
    if R32_MASK_LOGGED.swap(true, Ordering::Relaxed) {
        return;
    }
    let mut names: Vec<&str> = slots.iter().map(|&(_, w)| w).collect();
    names.sort_unstable();
    names.dedup();
    diag::warn!(
        World,
        "gpu: no 32-bit float filtering; {} texture slot(s) use a placeholder instead of: {}",
        slots.len(),
        names.join(", ")
    );
}

const fn dimension_lane(dimension: SamplerTextureDimension) -> usize {
    match dimension {
        SamplerTextureDimension::D2 => 0,
        SamplerTextureDimension::Cube => 1,
        SamplerTextureDimension::D3 => 2,
    }
}

const fn capacity(dimension: SamplerTextureDimension) -> u32 {
    match dimension {
        SamplerTextureDimension::D2 => TEXTURE_TABLE_2D_CAPACITY,
        SamplerTextureDimension::Cube => TEXTURE_TABLE_CUBE_CAPACITY,
        SamplerTextureDimension::D3 => TEXTURE_TABLE_3D_CAPACITY,
    }
}

impl ExactTextureTable {
    pub fn epoch(&self) -> TableEpoch {
        self.epoch
    }

    /// Lets the table write the white placeholder's texel. Called once per
    /// frame by the prepare system, which owns the queue.
    pub fn attach_queue(&mut self, queue: &RenderQueue) {
        if self.queue.is_none() {
            self.queue = Some(queue.clone());
        }
    }

    fn white_placeholder(&mut self, device: &RenderDevice) -> TextureView {
        if let Some(view) = &self.white_2d {
            return view.clone();
        }
        let descriptor = TextureDescriptor {
            label: Some("iw4_texture_table_placeholder_white"),
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        };
        let texture = match &self.queue {
            Some(queue) => device.create_texture_with_data(
                queue,
                &descriptor,
                bevy::render::render_resource::TextureDataOrder::default(),
                &[255],
            ),
            // No queue yet: a zero texel still makes the bind group valid.
            None => device.create_texture(&descriptor),
        };
        let view = texture.create_view(&TextureViewDescriptor::default());
        // Only cache a fully written texel, so a later frame can retry.
        if self.queue.is_some() {
            self.white_2d = Some(view.clone());
        }
        view
    }

    pub fn open_epoch(&mut self, epoch: TableEpoch) {
        if self.epoch == epoch {
            return;
        }
        self.epoch = epoch;
        for lane in &mut self.views {
            lane.clear();
        }
        self.view_index.clear();
        self.sun_shadow_indices.clear();
        self.r32_float_slots.clear();
        self.samplers.clear();
        self.sampler_index.clear();
        self.binds = None;
    }

    pub fn slot_word(
        &mut self,
        device: &RenderDevice,
        lane: &UploadedTextureBind,
    ) -> Result<u32, TextureTableRefusal> {
        self.ensure_placeholders(device);
        let lane_i = dimension_lane(lane.dimension);
        let texture = match self.view_index.get(&lane.view.id()) {
            Some(&index) => index,
            None => {
                let cap = capacity(lane.dimension);
                let index = u32::try_from(self.views[lane_i].len()).expect("table index fits u32");
                if index >= cap {
                    return Err(TextureTableRefusal::TexturesFull {
                        dimension: lane.dimension,
                        capacity: cap,
                    });
                }
                let index = u16::try_from(index).expect("capacities fit the 16-bit slot half");
                self.views[lane_i].push(lane.view.clone());
                self.view_index.insert(lane.view.id(), index);
                if lane.identity == UploadedTextureIdentity::Code(super::CODE_TEXTURE_SHADOWMAP_SUN)
                {
                    self.sun_shadow_indices.push(index);
                }
                // Only the 2D lane holds these render targets (the mask edits `views[0]`).
                if lane_i == 0
                    && let Some(what) = r32_float_identity(lane.identity)
                {
                    self.r32_float_slots.push((index, what));
                }
                self.binds = None;
                index
            }
        };
        let key = SamplerKey::from(&lane.sampler);
        let sampler = match self.sampler_index.get(&key) {
            Some(&index) => index,
            None => {
                let index = u32::try_from(self.samplers.len()).expect("table index fits u32");
                if index >= TEXTURE_TABLE_SAMPLER_CAPACITY {
                    return Err(TextureTableRefusal::SamplersFull {
                        capacity: TEXTURE_TABLE_SAMPLER_CAPACITY,
                    });
                }
                let index = u16::try_from(index).expect("capacities fit the 16-bit slot half");
                self.samplers
                    .push(device.create_sampler(&lane.sampler.descriptor()));
                self.sampler_index.insert(key, index);
                self.binds = None;
                index
            }
        };
        Ok(d3d9_sm3::texture_slot_word(texture, sampler))
    }

    pub fn census(&self) -> (usize, usize, usize, usize) {
        let n = |lane: &Vec<TextureView>| lane.len().saturating_sub(1);
        (
            n(&self.views[0]),
            n(&self.views[1]),
            n(&self.views[2]),
            self.samplers.len().saturating_sub(1),
        )
    }

    pub fn binds(
        &mut self,
        device: &RenderDevice,
        registry: &super::exact_pipeline::ExactPipelineRegistry,
        layout: &BindGroupLayoutDescriptor,
    ) -> &TextureTableBinds {
        self.ensure_placeholders(device);
        if self.binds.is_none() {
            let bgl = registry.bind_group_layout(device, layout);
            let placeholder_2d = self.views[0][0].clone();
            // Created only where an R32Float slot has to be masked (not on desktop).
            let needs_white = !self.r32_float_slots.is_empty()
                && !device.features().contains(WgpuFeatures::FLOAT32_FILTERABLE);
            let white_2d = if needs_white {
                self.white_placeholder(device)
            } else {
                placeholder_2d.clone()
            };
            // Devices without 32-bit float filtering get a plain placeholder
            // in the slots that would hold an R32Float image, so the bind
            // group is valid. Those effects (dynamic shadows, float depth)
            // read the placeholder instead. Devices that do have the feature
            // are unchanged.
            let mask_r32 = !device.features().contains(WgpuFeatures::FLOAT32_FILTERABLE)
                && !self.r32_float_slots.is_empty();
            if mask_r32 {
                note_r32_masked(&self.r32_float_slots);
            }
            let scene = self.create_bind_group(
                device,
                &bgl,
                "iw4_texture_table_scene",
                None,
                mask_r32.then_some(&white_2d),
            );
            let sun_caster = if self.sun_shadow_indices.is_empty() {
                scene.clone()
            } else {
                self.create_bind_group(
                    device,
                    &bgl,
                    "iw4_texture_table_sun_caster",
                    Some(&placeholder_2d),
                    mask_r32.then_some(&white_2d),
                )
            };
            self.rebuild_n = self.rebuild_n.saturating_add(1);
            self.binds = Some(TextureTableBinds { scene, sun_caster });
        }
        self.binds.as_ref().expect("built above")
    }

    pub fn views_bind_group(
        &mut self,
        device: &RenderDevice,
        bgl: &bevy::render::render_resource::BindGroupLayout,
        label: &'static str,
        views: &[&TextureView],
        samplers: &[&Sampler],
    ) -> BindGroup {
        let views: Vec<&WgpuTextureView> = views.iter().map(|v| &***v).collect();
        let samplers: Vec<&WgpuSampler> = samplers.iter().map(|v| &***v).collect();
        self.ensure_placeholders(device);
        let placeholders = self.placeholders.as_ref().expect("built above");
        device.create_bind_group(
            label,
            bgl,
            &[
                BindGroupEntry {
                    binding: TEXTURE_TABLE_BINDING_2D,
                    resource: BindingResource::TextureViewArray(&views),
                },
                BindGroupEntry {
                    binding: TEXTURE_TABLE_BINDING_CUBE,
                    resource: BindingResource::TextureViewArray(&[&*placeholders.views[1]]),
                },
                BindGroupEntry {
                    binding: TEXTURE_TABLE_BINDING_3D,
                    resource: BindingResource::TextureViewArray(&[&*placeholders.views[2]]),
                },
                BindGroupEntry {
                    binding: d3d9_sm3::TEXTURE_TABLE_BINDING_SAMPLERS,
                    resource: BindingResource::SamplerArray(&samplers),
                },
            ],
        )
    }

    fn create_bind_group(
        &self,
        device: &RenderDevice,
        bgl: &bevy::render::render_resource::BindGroupLayout,
        label: &'static str,
        mask_sun_shadow_with: Option<&TextureView>,
        mask_r32_float_with: Option<&TextureView>,
    ) -> BindGroup {
        let mut views_2d: Vec<&WgpuTextureView> = self.views[0].iter().map(|v| &**v).collect();
        if let Some(placeholder) = mask_sun_shadow_with {
            for &index in &self.sun_shadow_indices {
                views_2d[usize::from(index)] = &**placeholder;
            }
        }
        if let Some(placeholder) = mask_r32_float_with {
            for &(index, _) in &self.r32_float_slots {
                views_2d[usize::from(index)] = &**placeholder;
            }
        }
        let views_cube: Vec<&WgpuTextureView> = self.views[1].iter().map(|v| &**v).collect();
        let views_3d: Vec<&WgpuTextureView> = self.views[2].iter().map(|v| &**v).collect();
        let samplers: Vec<&WgpuSampler> = self.samplers.iter().map(|s| &**s).collect();
        device.create_bind_group(
            label,
            bgl,
            &[
                BindGroupEntry {
                    binding: TEXTURE_TABLE_BINDING_2D,
                    resource: BindingResource::TextureViewArray(&views_2d),
                },
                BindGroupEntry {
                    binding: TEXTURE_TABLE_BINDING_CUBE,
                    resource: BindingResource::TextureViewArray(&views_cube),
                },
                BindGroupEntry {
                    binding: TEXTURE_TABLE_BINDING_3D,
                    resource: BindingResource::TextureViewArray(&views_3d),
                },
                BindGroupEntry {
                    binding: d3d9_sm3::TEXTURE_TABLE_BINDING_SAMPLERS,
                    resource: BindingResource::SamplerArray(&samplers),
                },
            ],
        )
    }

    fn ensure_placeholders(&mut self, device: &RenderDevice) {
        let placeholders = self.placeholders.get_or_insert_with(|| {
            let make = |label, layers, dimension, view_dimension| {
                let texture = device.create_texture(&TextureDescriptor {
                    label: Some(label),
                    size: Extent3d {
                        width: 1,
                        height: 1,
                        depth_or_array_layers: layers,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension,
                    format: TextureFormat::R8Unorm,
                    usage: TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                texture.create_view(&TextureViewDescriptor {
                    label: Some(label),
                    dimension: Some(view_dimension),
                    ..default()
                })
            };
            Placeholders {
                views: [
                    make(
                        "iw4_texture_table_placeholder_2d",
                        1,
                        TextureDimension::D2,
                        TextureViewDimension::D2,
                    ),
                    make(
                        "iw4_texture_table_placeholder_cube",
                        6,
                        TextureDimension::D2,
                        TextureViewDimension::Cube,
                    ),
                    make(
                        "iw4_texture_table_placeholder_3d",
                        1,
                        TextureDimension::D3,
                        TextureViewDimension::D3,
                    ),
                ],
                sampler: device.create_sampler(&SamplerDescriptor {
                    label: Some("iw4_texture_table_placeholder_sampler"),
                    ..default()
                }),
            }
        });
        for (lane, view) in self.views.iter_mut().zip(placeholders.views.iter()) {
            if lane.is_empty() {
                lane.push(view.clone());
            }
        }
        if self.samplers.is_empty() {
            self.samplers.push(placeholders.sampler.clone());
        }
    }
}

pub(super) fn array_count(count: u32) -> NonZeroU32 {
    NonZeroU32::new(count).expect("texture table capacities are non-zero")
}
