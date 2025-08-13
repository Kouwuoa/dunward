use ash::vk;
use color_eyre::Result;

pub struct DescriptorSetLayoutBuilder<'a> {
    bindings: Vec<vk::DescriptorSetLayoutBinding<'a>>,
    binding_flags: Vec<vk::DescriptorBindingFlags>,
    immutable_samplers: Vec<Option<Vec<vk::Sampler>>>,
}

impl DescriptorSetLayoutBuilder<'_> {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            binding_flags: Vec::new(),
            immutable_samplers: Vec::new(),
        }
    }

    pub fn add_binding(
        mut self,
        binding: u32,
        descriptor_type: vk::DescriptorType,
        descriptor_count: u32,
        stages: vk::ShaderStageFlags,
        binding_flags: vk::DescriptorBindingFlags,
        immutable_samplers: Option<Vec<vk::Sampler>>,
    ) -> Self {
        let binding = vk::DescriptorSetLayoutBinding::default()
            .binding(binding)
            .descriptor_type(descriptor_type)
            .descriptor_count(descriptor_count)
            .stage_flags(stages);

        self.bindings.push(binding);
        self.binding_flags.push(binding_flags);
        self.immutable_samplers.push(immutable_samplers);
        self
    }

    pub fn build(
        mut self,
        flags: vk::DescriptorSetLayoutCreateFlags,
        device: &ash::Device,
    ) -> Result<vk::DescriptorSetLayout> {
        self.bindings
            .iter_mut()
            .zip(self.immutable_samplers.iter())
            .for_each(|(binding, immutable_samplers)| {
                if let Some(immutable_samplers) = immutable_samplers {
                    binding.p_immutable_samplers = immutable_samplers.as_ptr();
                }
            });

        let mut binding_flags_info = vk::DescriptorSetLayoutBindingFlagsCreateInfoEXT::default()
            .binding_flags(&self.binding_flags);

        let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(&self.bindings)
            .flags(flags)
            .push_next(&mut binding_flags_info);

        Ok(unsafe { device.create_descriptor_set_layout(&layout_info, None)? })
    }
}
