use std::sync::Arc;

use bevy::math::{IVec2, Vec2};
use bevy_vulkano::{create_device_image, DeviceImageView};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer,
        PrimaryCommandBuffer,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::Queue,
    format::Format,
    pipeline::{ComputePipeline, Pipeline, PipelineBindPoint},
    sync::GpuFuture,
};

use crate::{
    utils::{create_compute_pipeline, image_desc_set, storage_buffer_desc},
    CANVAS_SIZE_X, CANVAS_SIZE_Y, KERNEL_SIZE_X, KERNEL_SIZE_Y,
};

/// Creates a grid with empty matter values
fn empty_grid(
    compute_queue: &Arc<Queue>,
    width: u32,
    height: u32,
) -> Arc<CpuAccessibleBuffer<[u32]>> {
    CpuAccessibleBuffer::from_iter(
        compute_queue.device().clone(),
        BufferUsage::all(),
        false,
        vec![0; (width * height) as usize],
    )
    .unwrap()
}

/// Cellular automata simulation pipeline
pub struct CASimulator {
    compute_queue: Arc<Queue>,
    color_pipeline: Arc<ComputePipeline>,
    matter_in: Arc<CpuAccessibleBuffer<[u32]>>,
    matter_out: Arc<CpuAccessibleBuffer<[u32]>>,
    image: DeviceImageView,
}

impl CASimulator {
    /// Create new simulator pipeline for a compute queue. Ensure that canvas sizes are divisible by kernel sizes so no pixel
    /// remains unsimulated.
    pub fn new(compute_queue: Arc<Queue>) -> CASimulator {
        // In order to not miss any pixels, the following must be true
        assert_eq!(CANVAS_SIZE_X % KERNEL_SIZE_X, 0);
        assert_eq!(CANVAS_SIZE_Y % KERNEL_SIZE_Y, 0);
        let matter_in = empty_grid(&compute_queue, CANVAS_SIZE_X, CANVAS_SIZE_Y);
        let matter_out = empty_grid(&compute_queue, CANVAS_SIZE_X, CANVAS_SIZE_Y);

        let spec_const = color_cs::SpecializationConstants {
            canvas_size_x: CANVAS_SIZE_X as i32,
            canvas_size_y: CANVAS_SIZE_Y as i32,
            empty_matter: 0,
            constant_3: KERNEL_SIZE_X,
            constant_4: KERNEL_SIZE_Y,
        };

        // Create pipelines
        let color_pipeline = {
            let color_shader = color_cs::load(compute_queue.device().clone()).unwrap();
            // This must match the shader & inputs in dispatch
            let descriptor_layout = [
                (0, storage_buffer_desc()),
                (1, storage_buffer_desc()),
                (2, image_desc_set()),
            ];
            create_compute_pipeline(
                compute_queue.clone(),
                color_shader.entry_point("main").unwrap(),
                descriptor_layout.to_vec(),
                &spec_const,
            )
        };
        // Create color image
        let image = create_device_image(
            compute_queue.clone(),
            [CANVAS_SIZE_X, CANVAS_SIZE_Y],
            Format::R8G8B8A8_UNORM,
        );
        CASimulator {
            compute_queue,
            color_pipeline,
            matter_in,
            matter_out,
            image,
        }
    }

    /// Get canvas image for rendering
    pub fn color_image(&self) -> DeviceImageView {
        self.image.clone()
    }

    /// Are we within simulation bounds?
    fn is_inside(&self, pos: IVec2) -> bool {
        pos.x >= 0 && pos.x < CANVAS_SIZE_X as i32 && pos.y >= 0 && pos.y < CANVAS_SIZE_Y as i32
    }

    /// Index to access our one dimensional grid with two dimensional position
    fn index(&self, pos: IVec2) -> usize {
        (pos.y * CANVAS_SIZE_Y as i32 + pos.x) as usize
    }

    /// Draw matter line with given radius
    pub fn draw_matter(&mut self, line: &[IVec2], radius: f32, matter: u32) {
        let mut matter_in = self.matter_in.write().unwrap();
        for &pos in line.iter() {
            if !self.is_inside(pos) {
                continue;
            }
            let y_start = pos.y - radius as i32;
            let y_end = pos.y + radius as i32;
            let x_start = pos.x - radius as i32;
            let x_end = pos.x + radius as i32;
            for y in y_start..=y_end {
                for x in x_start..=x_end {
                    let world_pos = Vec2::new(x as f32, y as f32);
                    if world_pos
                        .distance(Vec2::new(pos.x as f32, pos.y as f32))
                        .round()
                        <= radius
                    {
                        if self.is_inside([x, y].into()) {
                            // Draw
                            matter_in[self.index([x, y].into())] = matter;
                        }
                    }
                }
            }
        }
    }

    /// Step simulation
    pub fn step(&mut self) {
        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            self.compute_queue.device().clone(),
            self.compute_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        // Finally color the image
        self.dispatch(&mut command_buffer_builder, self.color_pipeline.clone());

        // Finish
        let command_buffer = command_buffer_builder.build().unwrap();
        let finished = command_buffer.execute(self.compute_queue.clone()).unwrap();
        let _fut = finished.then_signal_fence_and_flush().unwrap();
    }

    /// Append a pipeline dispatch to our command buffer
    fn dispatch(
        &mut self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        pipeline: Arc<ComputePipeline>,
    ) {
        let pipeline_layout = pipeline.layout();
        let desc_layout = pipeline_layout.set_layouts().get(0).unwrap();
        let set = PersistentDescriptorSet::new(desc_layout.clone(), [
            WriteDescriptorSet::buffer(0, self.matter_in.clone()),
            WriteDescriptorSet::buffer(1, self.matter_out.clone()),
            WriteDescriptorSet::image_view(2, self.image.clone()),
        ])
        .unwrap();
        builder
            .bind_pipeline_compute(pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Compute, pipeline_layout.clone(), 0, set)
            .dispatch([
                CANVAS_SIZE_X / KERNEL_SIZE_X,
                CANVAS_SIZE_Y / KERNEL_SIZE_Y,
                1,
            ])
            .unwrap();
    }
}

mod color_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/color.glsl"
    }
}
