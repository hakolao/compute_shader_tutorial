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

pub fn u32_rgba_to_u8_rgba(num: u32) -> [u8; 4] {
    let a = num & 255;
    let b = (num >> 8) & 255;
    let g = (num >> 16) & 255;
    let r = (num >> 24) & 255;
    [r as u8, g as u8, b as u8, a as u8]
}

pub fn cursor_to_world(window: &Window, camera_pos: Vec2, camera_scale: f32) -> Vec2 {
    (window.cursor_position().unwrap() - Vec2::new(window.width() / 2.0, window.height() / 2.0))
        * camera_scale
        - camera_pos
}

#[derive(Debug, Copy, Clone)]
pub struct MousePos {
    pub world: Vec2,
}

impl MousePos {
    /// Inverts y and adds half canvas to the position (pixel units)
    pub fn canvas_pos(&self) -> Vec2 {
        (self.world + Vec2::new(CANVAS_SIZE_X as f32 / 2.0, CANVAS_SIZE_Y as f32 / 2.0))
    }
}

pub fn get_canvas_line(prev: Option<MousePos>, current: MousePos) -> Vec<IVec2> {
    let canvas_pos = current.canvas_pos();
    let prev = if let Some(prev) = prev {
        prev.canvas_pos()
    } else {
        canvas_pos
    };
    line_drawing::Bresenham::new(
        (prev.x.round() as i32, prev.y.round() as i32),
        (canvas_pos.x.round() as i32, canvas_pos.y.round() as i32),
    )
    .into_iter()
    .map(|pos| IVec2::new(pos.0, pos.1))
    .collect::<Vec<IVec2>>()
}
