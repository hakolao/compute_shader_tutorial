use std::{collections::BTreeMap, iter::FromIterator, sync::Arc};

use bevy::prelude::*;
use vulkano::{
    descriptor_set::{
        layout::{
            DescriptorSetLayout, DescriptorSetLayoutBinding, DescriptorSetLayoutCreateInfo,
            DescriptorType,
        },
        PersistentDescriptorSet, WriteDescriptorSet,
    },
    device::{Device, Queue},
    image::ImageViewAbstract,
    pipeline::{
        layout::PipelineLayoutCreateInfo, ComputePipeline, GraphicsPipeline, Pipeline,
        PipelineLayout,
    },
    sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode},
    shader::{EntryPoint, ShaderStages, SpecializationConstants},
};

use crate::{CANVAS_SIZE_X, CANVAS_SIZE_Y};

/// Descriptor set layout binding information for storage buffer
pub fn storage_buffer_desc() -> DescriptorSetLayoutBinding {
    DescriptorSetLayoutBinding {
        descriptor_type: DescriptorType::StorageBuffer,
        descriptor_count: 1,
        variable_descriptor_count: false,
        stages: ShaderStages::all(),
        immutable_samplers: Vec::new(),
        _ne: Default::default(),
    }
}

/// Descriptor set layout binding information for image buffer
pub fn image_desc_set() -> DescriptorSetLayoutBinding {
    DescriptorSetLayoutBinding {
        descriptor_type: DescriptorType::StorageImage,
        descriptor_count: 1,
        variable_descriptor_count: false,
        stages: ShaderStages::all(),
        immutable_samplers: Vec::new(),
        _ne: Default::default(),
    }
}

/// Creates a compute pipeline from given shader, with given descriptor layout binding.
/// The intention here is that the descriptor layout should correspond the shader's layout.
/// Normally you would use `ComputePipeline::new`, which would generate layout for descriptor set
/// automatically. However, because I've split the shaders to various different shaders, each shader
/// that does not use a variable from my shared layout don't get the bindings for that specific variable.
/// See https://github.com/vulkano-rs/vulkano/pull/1778 and https://github.com/vulkano-rs/vulkano/issues/1556#issuecomment-821658405.
pub fn create_compute_pipeline<Css>(
    compute_queue: Arc<Queue>,
    shader_entry_point: EntryPoint,
    descriptor_layout: Vec<(u32, DescriptorSetLayoutBinding)>,
    specialization_constants: &Css,
) -> Arc<ComputePipeline>
where
    Css: SpecializationConstants,
{
    let push_constant_reqs = shader_entry_point
        .push_constant_requirements()
        .cloned()
        .into_iter()
        .collect();
    let set_layout = DescriptorSetLayout::new(
        compute_queue.device().clone(),
        DescriptorSetLayoutCreateInfo {
            bindings: BTreeMap::from_iter(descriptor_layout),
            ..Default::default()
        },
    )
    .unwrap();
    let pipeline_layout =
        PipelineLayout::new(compute_queue.device().clone(), PipelineLayoutCreateInfo {
            set_layouts: vec![set_layout],
            push_constant_ranges: push_constant_reqs,
            ..Default::default()
        })
        .unwrap();
    ComputePipeline::with_pipeline_layout(
        compute_queue.device().clone(),
        shader_entry_point,
        specialization_constants,
        pipeline_layout.clone(),
        None,
    )
    .unwrap()
}

/// Creates a descriptor set for sampled image descriptor set using nearest sampling. This means that the image
/// will be pixel perfect.
pub fn create_image_sampler_nearest_descriptor_set(
    device: Arc<Device>,
    pipeline: Arc<GraphicsPipeline>,
    image: Arc<dyn ImageViewAbstract>,
) -> Arc<PersistentDescriptorSet> {
    let layout = pipeline.layout().set_layouts().get(0).unwrap();
    let sampler = Sampler::new(device, SamplerCreateInfo {
        mag_filter: Filter::Nearest,
        min_filter: Filter::Nearest,
        address_mode: [SamplerAddressMode::Repeat; 3],
        mipmap_mode: SamplerMipmapMode::Nearest,
        ..Default::default()
    })
    .unwrap();
    PersistentDescriptorSet::new(layout.clone(), [WriteDescriptorSet::image_view_sampler(
        0,
        image.clone(),
        sampler,
    )])
    .unwrap()
}

/// Converts u32 color to array of 4 u8
pub fn u32_rgba_to_u8_rgba(num: u32) -> [u8; 4] {
    let a = num & 255;
    let b = (num >> 8) & 255;
    let g = (num >> 16) & 255;
    let r = (num >> 24) & 255;
    [r as u8, g as u8, b as u8, a as u8]
}

/// Converts array of 4 u8 colors to u32
pub fn u8_rgba_to_u32_rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32 & 255)
}

/// Converts u32 color to gray scale for a wanted visual effect
/// https://stackoverflow.com/questions/42516203/converting-rgba-image-to-grayscale-golang
pub fn grey_scale_u32(color: u32) -> u32 {
    let color = u32_rgba_to_u8_rgba(color);
    let r = (0.299 * color[0] as f32) as u8;
    let g = (0.587 * color[1] as f32) as u8;
    let b = (0.114 * color[2] as f32) as u8;
    let y = r + g + b;
    u8_rgba_to_u32_rgba(y, y, y, 255)
}

/// Converts cursor position to world coordinates
pub fn cursor_to_world(window: &Window, camera_pos: Vec2, camera_scale: f32) -> Vec2 {
    (window.cursor_position().unwrap() - Vec2::new(window.width() / 2.0, window.height() / 2.0))
        * camera_scale
        - camera_pos
}

/// Mouse world position
#[derive(Debug, Copy, Clone)]
pub struct MousePos {
    pub world: Vec2,
}

impl MousePos {
    pub fn new(pos: Vec2) -> MousePos {
        MousePos {
            world: pos,
        }
    }

    /// Converts world position to canvas position:
    /// Inverts y and adds half canvas to the position (pixel units)
    pub fn canvas_pos(&self) -> Vec2 {
        self.world + Vec2::new(CANVAS_SIZE_X as f32 / 2.0, CANVAS_SIZE_Y as f32 / 2.0)
    }
}
