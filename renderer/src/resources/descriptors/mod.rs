//! Vulkan descriptor set allocators, layout builders, and write helpers.

pub(crate) mod allocator;
pub(crate) mod layout_builder;
pub(crate) mod writer;

pub(crate) use allocator::DescriptorAllocator;
pub(crate) use layout_builder::DescriptorSetLayoutBuilder;
pub(crate) use writer::DescriptorWriter;
