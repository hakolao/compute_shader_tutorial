use std::sync::Arc;

use vulkano::{
    buffer::TypedBufferAccess,
    command_buffer::{
        AutoCommandBufferBuilder, CommandBufferInheritanceInfo, CommandBufferUsage,
        SecondaryAutoCommandBuffer,
    },
    device::Queue,
    image::{ImageAccess, ImageViewAbstract},
    pipeline::{
        graphics::{
            color_blend::ColorBlendState,
            input_assembly::InputAssemblyState,
            vertex_input::BuffersDefinition,
            viewport::{Viewport, ViewportState},
        },
        GraphicsPipeline, Pipeline, PipelineBindPoint,
    },
    render_pass::Subpass,
};

use crate::{
    camera::OrthographicCamera,
    utils::create_image_sampler_nearest_descriptor_set,
    vertex::{Mesh, TexturedQuad, TexturedVertex},
};

/// Pipeline to draw pixel perfect images on quads
pub struct DrawQuadPipeline {
    gfx_queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
    subpass: Subpass,
    quad: Mesh,
}

impl DrawQuadPipeline {
    pub fn new(gfx_queue: Arc<Queue>, subpass: Subpass) -> DrawQuadPipeline {
        let quad = TexturedQuad::new(1.0, 1.0, [1.0; 4]).to_mesh(gfx_queue.device().clone());
        let pipeline = {
            let vs = vs::load(gfx_queue.device().clone()).expect("failed to create shader module");
            let fs = fs::load(gfx_queue.device().clone()).expect("failed to create shader module");
            GraphicsPipeline::start()
                .vertex_input_state(BuffersDefinition::new().vertex::<TexturedVertex>())
                .vertex_shader(vs.entry_point("main").unwrap(), ())
                .input_assembly_state(InputAssemblyState::new())
                .fragment_shader(fs.entry_point("main").unwrap(), ())
                .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
                .render_pass(subpass.clone())
                .color_blend_state(ColorBlendState::default().blend_alpha())
                .build(gfx_queue.device().clone())
                .unwrap()
        };
        DrawQuadPipeline {
            gfx_queue,
            pipeline,
            subpass,
            quad,
        }
    }

    /// Draw input `image` on a quad at (0.0, 0.0), between -1.0 and 1.0
    pub fn draw(
        &mut self,
        viewport_dimensions: [u32; 2],
        camera: OrthographicCamera,
        image: Arc<dyn ImageViewAbstract>,
        flip_x: bool,
        flip_y: bool,
    ) -> SecondaryAutoCommandBuffer {
        // Command buffer for our single subpass
        let mut builder = AutoCommandBufferBuilder::secondary(
            self.gfx_queue.device().clone(),
            self.gfx_queue.family(),
            CommandBufferUsage::MultipleSubmit,
            CommandBufferInheritanceInfo {
                render_pass: Some(self.subpass.clone().into()),
                ..Default::default()
            },
        )
        .unwrap();

        let dims = image.image().dimensions();
        let push_constants = vs::ty::PushConstants {
            world_to_screen: camera.world_to_screen().to_cols_array_2d(),
            // Scale transforms our 1.0 sized quad to actual pixel size in screen space
            scale: [
                dims.width() as f32 * if flip_x { -1.0 } else { 1.0 },
                dims.height() as f32 * if flip_y { -1.0 } else { 1.0 },
            ],
        };

        let image_sampler_descriptor_set = create_image_sampler_nearest_descriptor_set(
            self.gfx_queue.device().clone(),
            self.pipeline.clone(),
            image,
        );
        builder
            .set_viewport(0, [Viewport {
                origin: [0.0, 0.0],
                dimensions: [viewport_dimensions[0] as f32, viewport_dimensions[1] as f32],
                depth_range: 0.0..1.0,
            }])
            .bind_pipeline_graphics(self.pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.pipeline.layout().clone(),
                0,
                image_sampler_descriptor_set,
            )
            .push_constants(self.pipeline.layout().clone(), 0, push_constants)
            .bind_vertex_buffers(0, self.quad.vertices.clone())
            .bind_index_buffer(self.quad.indices.clone())
            .draw_indexed(self.quad.indices.len() as u32, 1, 0, 0, 0)
            .unwrap();
        builder.build().unwrap()
    }
}

mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        path: "shaders/quad_vert.glsl"
    }
}

mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        path: "shaders/quad_frag.glsl"
    }
}
