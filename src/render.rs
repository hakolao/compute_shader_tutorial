use std::sync::Arc;

use bevy_vulkano::FinalImageView;
use vulkano::{
    command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, SubpassContents},
    device::Queue,
    format::Format,
    image::{ImageAccess, ImageViewAbstract},
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    sync::GpuFuture,
};

use crate::{camera::OrthographicCamera, quad_pipeline::DrawQuadPipeline};

/// A render pass which places an image over screen frame
pub struct FillScreenRenderPass {
    gfx_queue: Arc<Queue>,
    render_pass: Arc<RenderPass>,
    quad_pipeline: DrawQuadPipeline,
}

impl FillScreenRenderPass {
    pub fn new(gfx_queue: Arc<Queue>, output_format: Format) -> FillScreenRenderPass {
        let render_pass = vulkano::single_pass_renderpass!(gfx_queue.device().clone(),
            attachments: {
                color: {
                    // Image is cleared at the start of render pass
                    load: Clear,
                    store: Store,
                    format: output_format,
                    samples: 1,
                }
            },
            pass: {
                    color: [color],
                    depth_stencil: {}
            }
        )
        .unwrap();
        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();
        let quad_pipeline = DrawQuadPipeline::new(gfx_queue.clone(), subpass);
        FillScreenRenderPass {
            gfx_queue,
            render_pass,
            quad_pipeline,
        }
    }

    /// Place view exactly over swapchain image target.
    /// Texture draw pipeline uses a quad onto which it places the view.
    pub fn draw<F>(
        &mut self,
        before_future: F,
        camera: OrthographicCamera,
        image: Arc<dyn ImageViewAbstract>,
        target: FinalImageView,
        clear_color: [f32; 4],
        flip_x: bool,
        flip_y: bool,
    ) -> Box<dyn GpuFuture>
    where
        F: GpuFuture + 'static,
    {
        // Get dimensions of target image
        let target_image = target.image().dimensions();
        // Create framebuffer (must be in same order as render pass description in `new`)
        let framebuffer = Framebuffer::new(self.render_pass.clone(), FramebufferCreateInfo {
            attachments: vec![target],
            ..Default::default()
        })
        .unwrap();
        // Create primary command buffer builder & begin render pass with black clear color
        let mut command_buffer_builder = AutoCommandBufferBuilder::primary(
            self.gfx_queue.device().clone(),
            self.gfx_queue.family(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();
        command_buffer_builder
            .begin_render_pass(framebuffer, SubpassContents::SecondaryCommandBuffers, [
                clear_color.into(),
            ])
            .unwrap();
        // Create secondary command buffer from quad pipeline (subpass) and execute it inside our render pass.
        // Then build the primary command buffer and execute it.
        let cb =
            self.quad_pipeline
                .draw(target_image.width_height(), camera, image, flip_x, flip_y);
        command_buffer_builder.execute_commands(cb).unwrap();
        command_buffer_builder.end_render_pass().unwrap();
        let command_buffer = command_buffer_builder.build().unwrap();
        before_future
            .then_execute(self.gfx_queue.clone(), command_buffer)
            .unwrap()
            .boxed()
    }
}
