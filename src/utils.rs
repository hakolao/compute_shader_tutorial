use std::sync::Arc;

use vulkano::{
    descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet},
    device::Device,
    image::ImageViewAbstract,
    pipeline::{GraphicsPipeline, Pipeline},
    sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo, SamplerMipmapMode},
};

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
