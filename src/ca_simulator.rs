use std::sync::Arc;

use bevy::{
    math::{IVec2, Vec2},
    utils::HashMap,
};
use bevy_vulkano::{create_device_image, DeviceImageView};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, DeviceLocalBuffer},
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer,
        PrimaryCommandBuffer,
    },
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::Queue,
    format::Format,
    pipeline::{ComputePipeline, Pipeline, PipelineBindPoint},
    sync::GpuFuture,
    DeviceSize,
};

use crate::{
    matter::{MatterId, MatterWithColor},
    utils::{create_compute_pipeline, image_desc_set, storage_buffer_desc},
    CANVAS_SIZE_X, CANVAS_SIZE_Y, LOCAL_SIZE_X, LOCAL_SIZE_Y, NUM_WORK_GROUPS_X, NUM_WORK_GROUPS_Y,
};

fn device_grid(
    compute_queue: &Arc<Queue>,
    width: u32,
    height: u32,
) -> Arc<DeviceLocalBuffer<[u32]>> {
    DeviceLocalBuffer::array(
        compute_queue.device().clone(),
        (width * height) as DeviceSize,
        BufferUsage::storage_buffer() | BufferUsage::transfer_destination(),
        compute_queue.device().active_queue_families(),
    )
    .unwrap()
}

/// Cellular automata simulation pipeline
pub struct CASimulator {
    compute_queue: Arc<Queue>,
    fall_pipeline: Arc<ComputePipeline>,
    slide_pipeline: Arc<ComputePipeline>,
    color_pipeline: Arc<ComputePipeline>,
    draw_matter_pipeline: Arc<ComputePipeline>,
    draw_matter_staged: HashMap<IVec2, (MatterWithColor, f32)>,
    matter_in: Arc<DeviceLocalBuffer<[u32]>>,
    matter_out: Arc<DeviceLocalBuffer<[u32]>>,
    image: DeviceImageView,
    pub sim_step: u32,
    move_step: u32,
    pub dispatches_per_step: u32,
    draw_radius: f32,
    draw_matter: MatterWithColor,
    draw_pos: IVec2,
}

impl CASimulator {
    /// Create new simulator pipeline for a compute queue. Ensure that canvas sizes are divisible by kernel sizes so no pixel
    /// remains unsimulated.
    pub fn new(compute_queue: Arc<Queue>) -> CASimulator {
        // In order to not miss any pixels, the following must be true
        assert_eq!(CANVAS_SIZE_X % LOCAL_SIZE_X, 0);
        assert_eq!(CANVAS_SIZE_Y % LOCAL_SIZE_Y, 0);
        let matter_in = device_grid(&compute_queue, CANVAS_SIZE_X, CANVAS_SIZE_Y);
        let matter_out = device_grid(&compute_queue, CANVAS_SIZE_X, CANVAS_SIZE_Y);

        // Assumes all shaders that are loaded with specialication constants have the same constants
        let spec_const = fall_empty_cs::SpecializationConstants {
            canvas_size_x: CANVAS_SIZE_X as i32,
            canvas_size_y: CANVAS_SIZE_Y as i32,
            empty_matter: MatterWithColor::new(MatterId::Empty).value,
            constant_3: LOCAL_SIZE_X,
            constant_4: LOCAL_SIZE_Y,
        };

        // Create pipelines
        let (fall_pipeline, slide_pipeline, color_pipeline, draw_matter_pipeline) = {
            let fall_shader = fall_empty_cs::load(compute_queue.device().clone()).unwrap();
            let slide_shader = slide_down_empty_cs::load(compute_queue.device().clone()).unwrap();
            let color_shader = color_cs::load(compute_queue.device().clone()).unwrap();
            let draw_matter_shader = draw_matter_cs::load(compute_queue.device().clone()).unwrap();
            // This must match the shader & inputs in dispatch
            let descriptor_layout = [
                (0, storage_buffer_desc()),
                (1, storage_buffer_desc()),
                (2, image_desc_set()),
            ];
            (
                create_compute_pipeline(
                    compute_queue.clone(),
                    fall_shader.entry_point("main").unwrap(),
                    descriptor_layout.to_vec(),
                    &spec_const,
                ),
                create_compute_pipeline(
                    compute_queue.clone(),
                    slide_shader.entry_point("main").unwrap(),
                    descriptor_layout.to_vec(),
                    &spec_const,
                ),
                create_compute_pipeline(
                    compute_queue.clone(),
                    color_shader.entry_point("main").unwrap(),
                    descriptor_layout.to_vec(),
                    &spec_const,
                ),
                create_compute_pipeline(
                    compute_queue.clone(),
                    draw_matter_shader.entry_point("main").unwrap(),
                    descriptor_layout.to_vec(),
                    &spec_const,
                ),
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
            fall_pipeline,
            slide_pipeline,
            color_pipeline,
            draw_matter_pipeline,
            draw_matter_staged: HashMap::default(),
            matter_in,
            matter_out,
            image,
            sim_step: 0,
            move_step: 0,
            dispatches_per_step: 0,
            draw_radius: 0.0,
            draw_matter: MatterWithColor::from(0),
            draw_pos: IVec2::new(0, 0),
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

    // /// Query matter at pos
    // pub fn query_matter(&self, pos: IVec2) -> Option<MatterId> {
    //     if self.is_inside(pos) {
    //         let matter_in = self.matter_in.read().unwrap();
    //         let index = self.index(pos);
    //         Some(MatterWithColor::from(matter_in[index]).matter_id())
    //     } else {
    //         None
    //     }
    // }

    /// Draw matter line with given radius
    pub fn draw_matter(&mut self, line: &[IVec2], radius: f32, matter: MatterId) {
        for &pos in line.iter() {
            if !self.is_inside(pos) {
                continue;
            }
            self.draw_matter_staged
                .insert(pos, (MatterWithColor::new(matter), radius));
        }
    }

    fn dispatch_draw_matter(
        &mut self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        pipeline: Arc<ComputePipeline>,
    ) {
        let draw_matter_staged = self.draw_matter_staged.clone();
        for (pos, (matter, radius)) in draw_matter_staged.iter() {
            self.draw_pos = *pos;
            self.draw_matter = *matter;
            self.draw_radius = *radius;
            self.dispatch(builder, pipeline.clone(), false);
        }
    }

    /// Step simulation
    pub fn step(&mut self, move_steps: u32, is_paused: bool) {
        self.dispatches_per_step = 0;

        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            self.compute_queue.device().clone(),
            self.compute_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        if !self.draw_matter_staged.is_empty() {
            self.dispatch_draw_matter(
                &mut command_buffer_builder,
                self.draw_matter_pipeline.clone(),
            );
            self.draw_matter_staged.clear();
        }

        if !is_paused {
            for _ in 0..move_steps {
                self.step_movement(&mut command_buffer_builder, self.fall_pipeline.clone());
                self.step_movement(&mut command_buffer_builder, self.slide_pipeline.clone());
            }
        }

        // Finally color the image
        self.dispatch(
            &mut command_buffer_builder,
            self.color_pipeline.clone(),
            false,
        );

        // Finish
        let command_buffer = command_buffer_builder.build().unwrap();
        let finished = command_buffer.execute(self.compute_queue.clone()).unwrap();
        let _fut = finished.then_signal_fence_and_flush().unwrap();

        self.sim_step += 1;
    }

    /// Step a movement pipeline. move_step affects the order of sliding direction
    fn step_movement(
        &mut self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        pipeline: Arc<ComputePipeline>,
    ) {
        self.dispatch(builder, pipeline.clone(), true);
        self.move_step += 1;
    }

    /// Append a pipeline dispatch to our command buffer
    fn dispatch(
        &mut self,
        builder: &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
        pipeline: Arc<ComputePipeline>,
        swap: bool,
    ) {
        let pipeline_layout = pipeline.layout();
        let desc_layout = pipeline_layout.set_layouts().get(0).unwrap();
        let set = PersistentDescriptorSet::new(desc_layout.clone(), [
            WriteDescriptorSet::buffer(0, self.matter_in.clone()),
            WriteDescriptorSet::buffer(1, self.matter_out.clone()),
            WriteDescriptorSet::image_view(2, self.image.clone()),
        ])
        .unwrap();
        // Assumes all shaders that are 'dispatched' have the same push constants
        let push_constants = fall_empty_cs::ty::PushConstants {
            sim_step: self.sim_step as u32,
            move_step: self.move_step as u32,
            draw_pos: self.draw_pos.into(),
            draw_radius: self.draw_radius,
            draw_matter: self.draw_matter.value,
        };
        builder
            .bind_pipeline_compute(pipeline.clone())
            .bind_descriptor_sets(PipelineBindPoint::Compute, pipeline_layout.clone(), 0, set)
            .push_constants(pipeline_layout.clone(), 0, push_constants)
            .dispatch([NUM_WORK_GROUPS_X, NUM_WORK_GROUPS_Y, 1])
            .unwrap();

        // Double buffering: Swap input and output so the output becomes the input for next frame
        if swap {
            std::mem::swap(&mut self.matter_in, &mut self.matter_out);
        }
        self.dispatches_per_step += 1;
    }
}

mod fall_empty_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/fall_empty.glsl"
    }
}

mod slide_down_empty_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/slide_down_empty.glsl"
    }
}

mod color_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/color.glsl"
    }
}

mod draw_matter_cs {
    vulkano_shaders::shader! {
        ty: "compute",
        path: "compute_shaders/draw_matter.glsl"
    }
}

// Most of the tests in a simple project like this can probably be done visually... If it renders right, it's right.
// However, I'll show here how you can test your shader & compute pass logic. And as the project grows
// you'll want to be doing more unit testing...
#[cfg(test)]
mod tests {
    use bevy::math::IVec2;
    use bevy_vulkano::{VulkanoContext, VulkanoWinitConfig};

    use crate::{ca_simulator::CASimulator, matter::MatterId};

    fn test_setup() -> (VulkanoContext, CASimulator) {
        // Create vulkano context
        let vulkano_context = VulkanoContext::new(&VulkanoWinitConfig {
            add_primary_window: false,
            ..VulkanoWinitConfig::default()
        });
        // Create Simulation pipeline
        let simulator = CASimulator::new(vulkano_context.compute_queue());
        (vulkano_context, simulator)
    }

    // #[test]
    // fn test_example_sandfall() {
    //     let (_ctx, mut simulator) = test_setup();
    //     let pos = IVec2::new(10, 10);
    //     // Empty matter first
    //     assert_eq!(simulator.query_matter(pos), Some(MatterId::Empty));
    //     simulator.draw_matter(&[pos], 0.5, MatterId::Sand);
    //     // After drawing, We have Sand
    //     assert_eq!(simulator.query_matter(pos), Some(MatterId::Sand));
    //     // Step once
    //     simulator.step(1, false);
    //     // Old position is empty
    //     assert_eq!(simulator.query_matter(pos), Some(MatterId::Empty));
    //     // New position under has Sand
    //     assert_eq!(
    //         simulator.query_matter(pos + IVec2::new(0, -1)),
    //         Some(MatterId::Sand)
    //     );
    // }
}
