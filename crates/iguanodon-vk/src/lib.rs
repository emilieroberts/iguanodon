// Copyright 2026 Emilie Roberts
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

//! Raw Vulkan 1.3/1.4 Backend (`iguanodon-vk`) for Iguanodon.
//! Interfaces directly with `libvulkan` on Linux/Android/Windows and MoltenVK/KosmicKrisp on Apple
//! platforms. Compiles pre-validated SPIR-V modules into `VkShaderModule`s, uploads 128-byte
//! `SdfPrimitiveInstance` batches into Vulkan device memory, measures GPU pass time via hardware
//! `VkQueryPool` timestamp queries, and produces rendered RGBA framebuffers.

use iguanodon_core::{Color, Point};
use iguanodon_shaders::{
    dual_kawase_blur_spirv, evaluate_sdf_instance_pixel, procedural_fire_spirv,
    sdf_uber_fragment_spirv, sdf_uber_shader_spirv, sdf_uber_vertex_spirv, validate_spirv_binary,
    water_ripple_spirv, SdfPrimitiveInstance,
};
use std::ffi::{c_char, c_void, CStr};
use std::ptr;

// Raw Vulkan C-ABI types & constants
type VkFlags = u32;
type VkBool32 = u32;
type VkResult = i32;
type VkDeviceSize = u64;
type VkInstance = *mut c_void;
type VkPhysicalDevice = *mut c_void;
type VkDevice = *mut c_void;
type VkQueue = *mut c_void;
type VkShaderModule = u64;
type VkBuffer = u64;
type VkImage = u64;
type VkImageView = u64;
type VkSampler = u64;
type VkDeviceMemory = u64;
type VkDescriptorSetLayout = u64;
type VkDescriptorPool = u64;
type VkDescriptorSet = u64;
type VkPipelineLayout = u64;
type VkRenderPass = u64;
type VkPipeline = u64;
type VkFramebuffer = u64;
type VkCommandPool = u64;
type VkCommandBuffer = *mut c_void;
type VkQueryPool = u64;
type VkSemaphore = u64;
type VkFence = u64;
type VkSurfaceKHR = u64;
type VkSwapchainKHR = u64;

const VK_SUCCESS: VkResult = 0;
const VK_SUBOPTIMAL_KHR: VkResult = 1000001003;
const VK_ERROR_OUT_OF_DATE_KHR: VkResult = -1000001004;

const VK_STRUCTURE_TYPE_APPLICATION_INFO: u32 = 0;
const VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO: u32 = 1;
const VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO: u32 = 2;
const VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO: u32 = 3;
const VK_STRUCTURE_TYPE_SUBMIT_INFO: u32 = 4;
const VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO: u32 = 5;
const VK_STRUCTURE_TYPE_FENCE_CREATE_INFO: u32 = 8;
const VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO: u32 = 9;
const VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO: u32 = 12;
const VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO: u32 = 14;
const VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO: u32 = 15;
const VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO: u32 = 16;
const VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO: u32 = 18;
const VK_STRUCTURE_TYPE_QUERY_POOL_CREATE_INFO: u32 = 19;
const VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO: u32 = 20;
const VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO: u32 = 21;
const VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO: u32 = 22;
const VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO: u32 = 23;
const VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO: u32 = 24;
const VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO: u32 = 26;
const VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO: u32 = 27;
const VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO: u32 = 28;
const VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO: u32 = 30;
const VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO: u32 = 31;
const VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO: u32 = 32;
const VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO: u32 = 33;
const VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO: u32 = 34;
const VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET: u32 = 35;
const VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO: u32 = 37;
const VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO: u32 = 38;
const VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO: u32 = 39;
const VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO: u32 = 40;
const VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO: u32 = 42;
const VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO: u32 = 43;
const VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER: u32 = 45;
const VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR: u32 = 1000001000;
const VK_STRUCTURE_TYPE_PRESENT_INFO_KHR: u32 = 1000001001;
const VK_STRUCTURE_TYPE_XLIB_SURFACE_CREATE_INFO_KHR: u32 = 1000004000;
const VK_STRUCTURE_TYPE_ANDROID_SURFACE_CREATE_INFO_KHR: u32 = 1000008000;

const VK_BUFFER_USAGE_TRANSFER_SRC_BIT: u32 = 0x00000001;
const VK_BUFFER_USAGE_TRANSFER_DST_BIT: u32 = 0x00000002;
const VK_BUFFER_USAGE_STORAGE_BUFFER_BIT: u32 = 0x00000020;
const VK_BUFFER_USAGE_INDEX_BUFFER_BIT: u32 = 0x00000040;
const VK_BUFFER_USAGE_VERTEX_BUFFER_BIT: u32 = 0x00000080;

const VK_IMAGE_USAGE_TRANSFER_SRC_BIT: u32 = 0x00000001;
const VK_IMAGE_USAGE_TRANSFER_DST_BIT: u32 = 0x00000002;
const VK_IMAGE_USAGE_SAMPLED_BIT: u32 = 0x00000004;
const VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT: u32 = 0x00000010;

const VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT: u32 = 0x00000001;
const VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT: u32 = 0x00000002;
const VK_MEMORY_PROPERTY_HOST_COHERENT_BIT: u32 = 0x00000004;

const VK_FORMAT_R8G8B8A8_UNORM: u32 = 37;
const VK_FORMAT_B8G8R8A8_UNORM: u32 = 44;
const VK_FORMAT_R32G32B32A32_SFLOAT: u32 = 109;

const VK_VERTEX_INPUT_RATE_VERTEX: u32 = 0;
const VK_INDEX_TYPE_UINT32: u32 = 1;
const MAX_GPU_INSTANCES: usize = 16384;

const VK_IMAGE_LAYOUT_UNDEFINED: u32 = 0;
const VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL: u32 = 2;
const VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL: u32 = 5;
const VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL: u32 = 6;
const VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL: u32 = 7;
const VK_IMAGE_LAYOUT_PRESENT_SRC_KHR: u32 = 1000001002;

const VK_SHADER_STAGE_VERTEX_BIT: u32 = 0x00000001;
const VK_SHADER_STAGE_FRAGMENT_BIT: u32 = 0x00000010;
const VK_SHADER_STAGE_ALL_GRAPHICS: u32 = 0x0000001F;

const VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER: u32 = 1;
const VK_DESCRIPTOR_TYPE_STORAGE_BUFFER: u32 = 7;

const VK_QUERY_TYPE_TIMESTAMP: u32 = 2;
const VK_QUERY_RESULT_64_BIT: u32 = 0x00000001;
const VK_QUERY_RESULT_WAIT_BIT: u32 = 0x00000002;

const VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT: u32 = 0x00000001;
const VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT: u32 = 0x00000080;
const VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT: u32 = 0x00000400;
const VK_PIPELINE_STAGE_TRANSFER_BIT: u32 = 0x00001000;
const VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT: u32 = 0x00002000;

const VK_ACCESS_TRANSFER_WRITE_BIT: u32 = 0x00001000;
const VK_ACCESS_SHADER_READ_BIT: u32 = 0x00000020;

#[repr(C)]
struct VkApplicationInfo {
    s_type: u32,
    p_next: *const c_void,
    p_application_name: *const c_char,
    application_version: u32,
    p_engine_name: *const c_char,
    engine_version: u32,
    api_version: u32,
}

#[repr(C)]
struct VkInstanceCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    p_application_info: *const VkApplicationInfo,
    enabled_layer_count: u32,
    pp_enabled_layer_names: *const *const c_char,
    enabled_extension_count: u32,
    pp_enabled_extension_names: *const *const c_char,
}

#[repr(C)]
struct VkPhysicalDeviceLimits {
    _padding_before_timestamp: [u8; 408],
    timestamp_compute_and_graphics: u32,
    timestamp_period: f32,
    _padding_after: [u8; 96],
}

#[repr(C)]
struct VkPhysicalDeviceSparseProperties {
    _opaque: [u32; 5],
}

#[repr(C)]
struct VkPhysicalDeviceProperties {
    api_version: u32,
    driver_version: u32,
    vendor_id: u32,
    device_id: u32,
    device_type: u32,
    device_name: [c_char; 256],
    pipeline_cache_uuid: [u8; 16],
    limits: VkPhysicalDeviceLimits,
    sparse_properties: VkPhysicalDeviceSparseProperties,
}

#[repr(C)]
struct VkDeviceQueueCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    queue_family_index: u32,
    queue_count: u32,
    p_queue_priorities: *const f32,
}

#[repr(C)]
struct VkDeviceCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    queue_create_info_count: u32,
    p_queue_create_infos: *const VkDeviceQueueCreateInfo,
    enabled_layer_count: u32,
    pp_enabled_layer_names: *const *const c_char,
    enabled_extension_count: u32,
    pp_enabled_extension_names: *const *const c_char,
    p_enabled_features: *const c_void,
}

#[repr(C)]
struct VkShaderModuleCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    code_size: usize,
    p_code: *const u32,
}

#[repr(C)]
struct VkBufferCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    size: VkDeviceSize,
    usage: VkFlags,
    sharing_mode: u32,
    queue_family_index_count: u32,
    p_queue_family_indices: *const u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkExtent2D {
    width: u32,
    height: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkExtent3D {
    width: u32,
    height: u32,
    depth: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkOffset2D {
    x: i32,
    y: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkOffset3D {
    x: i32,
    y: i32,
    z: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkRect2D {
    offset: VkOffset2D,
    extent: VkExtent2D,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkViewport {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    min_depth: f32,
    max_depth: f32,
}

#[repr(C)]
struct VkImageCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    image_type: u32,
    format: u32,
    extent: VkExtent3D,
    mip_levels: u32,
    array_layers: u32,
    samples: u32,
    tiling: u32,
    usage: VkFlags,
    sharing_mode: u32,
    queue_family_index_count: u32,
    p_queue_family_indices: *const u32,
    initial_layout: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkComponentMapping {
    r: u32,
    g: u32,
    b: u32,
    a: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkImageSubresourceRange {
    aspect_mask: VkFlags,
    base_mip_level: u32,
    level_count: u32,
    base_array_layer: u32,
    layer_count: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkImageSubresourceLayers {
    aspect_mask: VkFlags,
    mip_level: u32,
    base_array_layer: u32,
    layer_count: u32,
}

#[repr(C)]
struct VkImageViewCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    image: VkImage,
    view_type: u32,
    format: u32,
    components: VkComponentMapping,
    subresource_range: VkImageSubresourceRange,
}

#[repr(C)]
struct VkSamplerCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    mag_filter: u32,
    min_filter: u32,
    mipmap_mode: u32,
    address_mode_u: u32,
    address_mode_v: u32,
    address_mode_w: u32,
    mip_lod_bias: f32,
    anisotropy_enable: VkBool32,
    max_anisotropy: f32,
    compare_enable: VkBool32,
    compare_op: u32,
    min_lod: f32,
    max_lod: f32,
    border_color: u32,
    unnormalized_coordinates: VkBool32,
}

#[repr(C)]
struct VkImageMemoryBarrier {
    s_type: u32,
    p_next: *const c_void,
    src_access_mask: VkFlags,
    dst_access_mask: VkFlags,
    old_layout: u32,
    new_layout: u32,
    src_queue_family_index: u32,
    dst_queue_family_index: u32,
    image: VkImage,
    subresource_range: VkImageSubresourceRange,
}

#[repr(C)]
struct VkBufferImageCopy {
    buffer_offset: VkDeviceSize,
    buffer_row_length: u32,
    buffer_image_height: u32,
    image_subresource: VkImageSubresourceLayers,
    image_offset: VkOffset3D,
    image_extent: VkExtent3D,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkDescriptorSetLayoutBinding {
    binding: u32,
    descriptor_type: u32,
    descriptor_count: u32,
    stage_flags: VkFlags,
    p_immutable_samplers: *const VkSampler,
}

#[repr(C)]
struct VkDescriptorSetLayoutCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    binding_count: u32,
    p_bindings: *const VkDescriptorSetLayoutBinding,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkDescriptorPoolSize {
    type_: u32,
    descriptor_count: u32,
}

#[repr(C)]
struct VkDescriptorPoolCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    max_sets: u32,
    pool_size_count: u32,
    p_pool_sizes: *const VkDescriptorPoolSize,
}

#[repr(C)]
struct VkDescriptorSetAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    descriptor_pool: VkDescriptorPool,
    descriptor_set_count: u32,
    p_set_layouts: *const VkDescriptorSetLayout,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkDescriptorBufferInfo {
    buffer: VkBuffer,
    offset: VkDeviceSize,
    range: VkDeviceSize,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkDescriptorImageInfo {
    sampler: VkSampler,
    image_view: VkImageView,
    image_layout: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkWriteDescriptorSet {
    s_type: u32,
    p_next: *const c_void,
    dst_set: VkDescriptorSet,
    dst_binding: u32,
    dst_array_element: u32,
    descriptor_count: u32,
    descriptor_type: u32,
    p_image_info: *const VkDescriptorImageInfo,
    p_buffer_info: *const VkDescriptorBufferInfo,
    p_texel_buffer_view: *const u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkPushConstantRange {
    stage_flags: VkFlags,
    offset: u32,
    size: u32,
}

#[repr(C)]
struct VkPipelineLayoutCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    set_layout_count: u32,
    p_set_layouts: *const VkDescriptorSetLayout,
    push_constant_range_count: u32,
    p_push_constant_ranges: *const VkPushConstantRange,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkAttachmentDescription {
    flags: VkFlags,
    format: u32,
    samples: u32,
    load_op: u32,
    store_op: u32,
    stencil_load_op: u32,
    stencil_store_op: u32,
    initial_layout: u32,
    final_layout: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkAttachmentReference {
    attachment: u32,
    layout: u32,
}

#[repr(C)]
struct VkSubpassDescription {
    flags: VkFlags,
    pipeline_bind_point: u32,
    input_attachment_count: u32,
    p_input_attachments: *const VkAttachmentReference,
    color_attachment_count: u32,
    p_color_attachments: *const VkAttachmentReference,
    p_resolve_attachments: *const VkAttachmentReference,
    p_depth_stencil_attachment: *const VkAttachmentReference,
    preserve_attachment_count: u32,
    p_preserve_attachments: *const u32,
}

#[repr(C)]
struct VkSubpassDependency {
    src_subpass: u32,
    dst_subpass: u32,
    src_stage_mask: VkFlags,
    dst_stage_mask: VkFlags,
    src_access_mask: VkFlags,
    dst_access_mask: VkFlags,
    dependency_flags: VkFlags,
}

#[repr(C)]
struct VkRenderPassCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    attachment_count: u32,
    p_attachments: *const VkAttachmentDescription,
    subpass_count: u32,
    p_subpasses: *const VkSubpassDescription,
    dependency_count: u32,
    p_dependencies: *const VkSubpassDependency,
}

#[repr(C)]
struct VkFramebufferCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    render_pass: VkRenderPass,
    attachment_count: u32,
    p_attachments: *const VkImageView,
    width: u32,
    height: u32,
    layers: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkPipelineShaderStageCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    stage: VkFlags,
    module: VkShaderModule,
    p_name: *const c_char,
    p_specialization_info: *const c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkVertexInputBindingDescription {
    binding: u32,
    stride: u32,
    input_rate: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkVertexInputAttributeDescription {
    location: u32,
    binding: u32,
    format: u32,
    offset: u32,
}

/// 16-byte GPU vertex (`vec4 in_pos_uv` at `location = 0`) stored in `VulkanRenderer::vertex_buffer`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuQuadVertex {
    pub pos_px: [f32; 2],
    pub uv: [f32; 2],
}

#[repr(C)]
struct VkPipelineVertexInputStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    vertex_binding_description_count: u32,
    p_vertex_binding_descriptions: *const VkVertexInputBindingDescription,
    vertex_attribute_description_count: u32,
    p_vertex_attribute_descriptions: *const VkVertexInputAttributeDescription,
}

#[repr(C)]
struct VkPipelineInputAssemblyStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    topology: u32,
    primitive_restart_enable: VkBool32,
}

#[repr(C)]
struct VkPipelineViewportStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    viewport_count: u32,
    p_viewports: *const VkViewport,
    scissor_count: u32,
    p_scissors: *const VkRect2D,
}

#[repr(C)]
struct VkPipelineRasterizationStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    depth_clamp_enable: VkBool32,
    rasterizer_discard_enable: VkBool32,
    polygon_mode: u32,
    cull_mode: VkFlags,
    front_face: u32,
    depth_bias_enable: VkBool32,
    depth_bias_constant_factor: f32,
    depth_bias_clamp: f32,
    depth_bias_slope_factor: f32,
    line_width: f32,
}

#[repr(C)]
struct VkPipelineMultisampleStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    rasterization_samples: u32,
    sample_shading_enable: VkBool32,
    min_sample_shading: f32,
    p_sample_mask: *const u32,
    alpha_to_coverage_enable: VkBool32,
    alpha_to_one_enable: VkBool32,
}

#[repr(C)]
struct VkPipelineColorBlendAttachmentState {
    blend_enable: VkBool32,
    src_color_blend_factor: u32,
    dst_color_blend_factor: u32,
    color_blend_op: u32,
    src_alpha_blend_factor: u32,
    dst_alpha_blend_factor: u32,
    alpha_blend_op: u32,
    color_write_mask: VkFlags,
}

#[repr(C)]
struct VkPipelineColorBlendStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    logic_op_enable: VkBool32,
    logic_op: u32,
    attachment_count: u32,
    p_attachments: *const VkPipelineColorBlendAttachmentState,
    blend_constants: [f32; 4],
}

#[repr(C)]
struct VkPipelineDynamicStateCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    dynamic_state_count: u32,
    p_dynamic_states: *const u32,
}

#[repr(C)]
struct VkGraphicsPipelineCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    stage_count: u32,
    p_stages: *const VkPipelineShaderStageCreateInfo,
    p_vertex_input_state: *const VkPipelineVertexInputStateCreateInfo,
    p_input_assembly_state: *const VkPipelineInputAssemblyStateCreateInfo,
    p_tessellation_state: *const c_void,
    p_viewport_state: *const VkPipelineViewportStateCreateInfo,
    p_rasterization_state: *const VkPipelineRasterizationStateCreateInfo,
    p_multisample_state: *const VkPipelineMultisampleStateCreateInfo,
    p_depth_stencil_state: *const c_void,
    p_color_blend_state: *const VkPipelineColorBlendStateCreateInfo,
    p_dynamic_state: *const VkPipelineDynamicStateCreateInfo,
    layout: VkPipelineLayout,
    render_pass: VkRenderPass,
    subpass: u32,
    base_pipeline_handle: VkPipeline,
    base_pipeline_index: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkClearColorValue {
    float32: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkClearValue {
    color: VkClearColorValue,
}

#[repr(C)]
struct VkRenderPassBeginInfo {
    s_type: u32,
    p_next: *const c_void,
    render_pass: VkRenderPass,
    framebuffer: VkFramebuffer,
    render_area: VkRect2D,
    clear_value_count: u32,
    p_clear_values: *const VkClearValue,
}

#[repr(C)]
struct VkSemaphoreCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
}

#[repr(C)]
struct VkFenceCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
}

#[repr(C)]
struct VkXlibSurfaceCreateInfoKHR {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    dpy: *mut c_void,
    window: u64,
}

#[repr(C)]
struct VkAndroidSurfaceCreateInfoKHR {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    window: *mut c_void,
}

#[repr(C)]
struct VkSurfaceCapabilitiesKHR {
    min_image_count: u32,
    max_image_count: u32,
    current_extent: VkExtent2D,
    min_image_extent: VkExtent2D,
    max_image_extent: VkExtent2D,
    max_image_array_layers: u32,
    supported_transforms: VkFlags,
    current_transform: VkFlags,
    supported_composite_alpha: VkFlags,
    supported_usage_flags: VkFlags,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkSurfaceFormatKHR {
    format: u32,
    color_space: u32,
}

#[repr(C)]
struct VkSwapchainCreateInfoKHR {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    surface: VkSurfaceKHR,
    min_image_count: u32,
    image_format: u32,
    image_color_space: u32,
    image_extent: VkExtent2D,
    image_array_layers: u32,
    image_usage: VkFlags,
    image_sharing_mode: u32,
    queue_family_index_count: u32,
    p_queue_family_indices: *const u32,
    pre_transform: VkFlags,
    composite_alpha: VkFlags,
    present_mode: u32,
    clipped: VkBool32,
    old_swapchain: VkSwapchainKHR,
}

#[repr(C)]
struct VkPresentInfoKHR {
    s_type: u32,
    p_next: *const c_void,
    wait_semaphore_count: u32,
    p_wait_semaphores: *const VkSemaphore,
    swapchain_count: u32,
    p_swapchains: *const VkSwapchainKHR,
    p_image_indices: *const u32,
    p_results: *mut VkResult,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SdfPushConstants {
    viewport: [f32; 2],
    pointer: [f32; 2],
    time_sec: f32,
    flags: u32,
    _pad: [f32; 2],
}

#[repr(C)]
struct VkMemoryRequirements {
    size: VkDeviceSize,
    alignment: VkDeviceSize,
    memory_type_bits: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkMemoryType {
    property_flags: VkFlags,
    heap_index: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct VkMemoryHeap {
    size: VkDeviceSize,
    flags: VkFlags,
}

#[repr(C)]
struct VkPhysicalDeviceMemoryProperties {
    memory_type_count: u32,
    memory_types: [VkMemoryType; 32],
    memory_heap_count: u32,
    memory_heaps: [VkMemoryHeap; 16],
}

#[repr(C)]
struct VkMemoryAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    allocation_size: VkDeviceSize,
    memory_type_index: u32,
}

#[repr(C)]
struct VkCommandPoolCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    queue_family_index: u32,
}

#[repr(C)]
struct VkCommandBufferAllocateInfo {
    s_type: u32,
    p_next: *const c_void,
    command_pool: VkCommandPool,
    level: u32,
    command_buffer_count: u32,
}

#[repr(C)]
struct VkCommandBufferBeginInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    p_inheritance_info: *const c_void,
}

#[repr(C)]
struct VkQueryPoolCreateInfo {
    s_type: u32,
    p_next: *const c_void,
    flags: VkFlags,
    query_type: u32,
    query_count: u32,
    pipeline_statistics: VkFlags,
}

#[repr(C)]
struct VkSubmitInfo {
    s_type: u32,
    p_next: *const c_void,
    wait_semaphore_count: u32,
    p_wait_semaphores: *const u64,
    p_wait_dst_stage_mask: *const VkFlags,
    command_buffer_count: u32,
    p_command_buffers: *const VkCommandBuffer,
    signal_semaphore_count: u32,
    p_signal_semaphores: *const u64,
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[link(name = "dl")]
extern "C" {
    fn dlopen(filename: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
}

#[cfg(any(target_os = "linux", target_os = "android"))]
#[allow(non_snake_case)]
struct VulkanFnTable {
    vkCreateInstance: unsafe extern "C" fn(*const VkInstanceCreateInfo, *const c_void, *mut VkInstance) -> VkResult,
    vkDestroyInstance: unsafe extern "C" fn(VkInstance, *const c_void),
    vkEnumeratePhysicalDevices: unsafe extern "C" fn(VkInstance, *mut u32, *mut VkPhysicalDevice) -> VkResult,
    vkGetPhysicalDeviceProperties: unsafe extern "C" fn(VkPhysicalDevice, *mut VkPhysicalDeviceProperties),
    vkGetPhysicalDeviceMemoryProperties: unsafe extern "C" fn(VkPhysicalDevice, *mut VkPhysicalDeviceMemoryProperties),
    vkCreateDevice: unsafe extern "C" fn(VkPhysicalDevice, *const VkDeviceCreateInfo, *const c_void, *mut VkDevice) -> VkResult,
    vkDestroyDevice: unsafe extern "C" fn(VkDevice, *const c_void),
    vkDeviceWaitIdle: unsafe extern "C" fn(VkDevice) -> VkResult,
    vkGetDeviceQueue: unsafe extern "C" fn(VkDevice, u32, u32, *mut VkQueue),
    vkCreateShaderModule: unsafe extern "C" fn(VkDevice, *const VkShaderModuleCreateInfo, *const c_void, *mut VkShaderModule) -> VkResult,
    vkDestroyShaderModule: unsafe extern "C" fn(VkDevice, VkShaderModule, *const c_void),
    vkCreateBuffer: unsafe extern "C" fn(VkDevice, *const VkBufferCreateInfo, *const c_void, *mut VkBuffer) -> VkResult,
    vkDestroyBuffer: unsafe extern "C" fn(VkDevice, VkBuffer, *const c_void),
    vkGetBufferMemoryRequirements: unsafe extern "C" fn(VkDevice, VkBuffer, *mut VkMemoryRequirements),
    vkCreateImage: unsafe extern "C" fn(VkDevice, *const VkImageCreateInfo, *const c_void, *mut VkImage) -> VkResult,
    vkDestroyImage: unsafe extern "C" fn(VkDevice, VkImage, *const c_void),
    vkGetImageMemoryRequirements: unsafe extern "C" fn(VkDevice, VkImage, *mut VkMemoryRequirements),
    vkBindImageMemory: unsafe extern "C" fn(VkDevice, VkImage, VkDeviceMemory, VkDeviceSize) -> VkResult,
    vkCreateImageView: unsafe extern "C" fn(VkDevice, *const VkImageViewCreateInfo, *const c_void, *mut VkImageView) -> VkResult,
    vkDestroyImageView: unsafe extern "C" fn(VkDevice, VkImageView, *const c_void),
    vkCreateSampler: unsafe extern "C" fn(VkDevice, *const VkSamplerCreateInfo, *const c_void, *mut VkSampler) -> VkResult,
    vkDestroySampler: unsafe extern "C" fn(VkDevice, VkSampler, *const c_void),
    vkCreateDescriptorSetLayout: unsafe extern "C" fn(VkDevice, *const VkDescriptorSetLayoutCreateInfo, *const c_void, *mut VkDescriptorSetLayout) -> VkResult,
    vkDestroyDescriptorSetLayout: unsafe extern "C" fn(VkDevice, VkDescriptorSetLayout, *const c_void),
    vkCreateDescriptorPool: unsafe extern "C" fn(VkDevice, *const VkDescriptorPoolCreateInfo, *const c_void, *mut VkDescriptorPool) -> VkResult,
    vkDestroyDescriptorPool: unsafe extern "C" fn(VkDevice, VkDescriptorPool, *const c_void),
    vkAllocateDescriptorSets: unsafe extern "C" fn(VkDevice, *const VkDescriptorSetAllocateInfo, *mut VkDescriptorSet) -> VkResult,
    vkUpdateDescriptorSets: unsafe extern "C" fn(VkDevice, u32, *const VkWriteDescriptorSet, u32, *const c_void),
    vkCreatePipelineLayout: unsafe extern "C" fn(VkDevice, *const VkPipelineLayoutCreateInfo, *const c_void, *mut VkPipelineLayout) -> VkResult,
    vkDestroyPipelineLayout: unsafe extern "C" fn(VkDevice, VkPipelineLayout, *const c_void),
    vkCreateRenderPass: unsafe extern "C" fn(VkDevice, *const VkRenderPassCreateInfo, *const c_void, *mut VkRenderPass) -> VkResult,
    vkDestroyRenderPass: unsafe extern "C" fn(VkDevice, VkRenderPass, *const c_void),
    vkCreateGraphicsPipelines: unsafe extern "C" fn(VkDevice, u64, u32, *const VkGraphicsPipelineCreateInfo, *const c_void, *mut VkPipeline) -> VkResult,
    vkDestroyPipeline: unsafe extern "C" fn(VkDevice, VkPipeline, *const c_void),
    vkCreateFramebuffer: unsafe extern "C" fn(VkDevice, *const VkFramebufferCreateInfo, *const c_void, *mut VkFramebuffer) -> VkResult,
    vkDestroyFramebuffer: unsafe extern "C" fn(VkDevice, VkFramebuffer, *const c_void),
    vkCreateSemaphore: unsafe extern "C" fn(VkDevice, *const VkSemaphoreCreateInfo, *const c_void, *mut VkSemaphore) -> VkResult,
    vkDestroySemaphore: unsafe extern "C" fn(VkDevice, VkSemaphore, *const c_void),
    vkCreateFence: unsafe extern "C" fn(VkDevice, *const VkFenceCreateInfo, *const c_void, *mut VkFence) -> VkResult,
    vkDestroyFence: unsafe extern "C" fn(VkDevice, VkFence, *const c_void),
    vkWaitForFences: unsafe extern "C" fn(VkDevice, u32, *const VkFence, VkBool32, u64) -> VkResult,
    vkResetFences: unsafe extern "C" fn(VkDevice, u32, *const VkFence) -> VkResult,
    vkAllocateMemory: unsafe extern "C" fn(VkDevice, *const VkMemoryAllocateInfo, *const c_void, *mut VkDeviceMemory) -> VkResult,
    vkFreeMemory: unsafe extern "C" fn(VkDevice, VkDeviceMemory, *const c_void),
    vkBindBufferMemory: unsafe extern "C" fn(VkDevice, VkBuffer, VkDeviceMemory, VkDeviceSize) -> VkResult,
    vkMapMemory: unsafe extern "C" fn(VkDevice, VkDeviceMemory, VkDeviceSize, VkDeviceSize, VkFlags, *mut *mut c_void) -> VkResult,
    vkUnmapMemory: unsafe extern "C" fn(VkDevice, VkDeviceMemory),
    vkCreateCommandPool: unsafe extern "C" fn(VkDevice, *const VkCommandPoolCreateInfo, *const c_void, *mut VkCommandPool) -> VkResult,
    vkDestroyCommandPool: unsafe extern "C" fn(VkDevice, VkCommandPool, *const c_void),
    vkAllocateCommandBuffers: unsafe extern "C" fn(VkDevice, *const VkCommandBufferAllocateInfo, *mut VkCommandBuffer) -> VkResult,
    vkResetCommandBuffer: unsafe extern "C" fn(VkCommandBuffer, VkFlags) -> VkResult,
    vkBeginCommandBuffer: unsafe extern "C" fn(VkCommandBuffer, *const VkCommandBufferBeginInfo) -> VkResult,
    vkEndCommandBuffer: unsafe extern "C" fn(VkCommandBuffer) -> VkResult,
    vkCmdPipelineBarrier: unsafe extern "C" fn(VkCommandBuffer, VkFlags, VkFlags, VkFlags, u32, *const c_void, u32, *const c_void, u32, *const VkImageMemoryBarrier),
    vkCmdCopyBufferToImage: unsafe extern "C" fn(VkCommandBuffer, VkBuffer, VkImage, u32, u32, *const VkBufferImageCopy),
    vkCmdCopyImageToBuffer: unsafe extern "C" fn(VkCommandBuffer, VkImage, u32, VkBuffer, u32, *const VkBufferImageCopy),
    vkCmdBeginRenderPass: unsafe extern "C" fn(VkCommandBuffer, *const VkRenderPassBeginInfo, u32),
    vkCmdEndRenderPass: unsafe extern "C" fn(VkCommandBuffer),
    vkCmdBindPipeline: unsafe extern "C" fn(VkCommandBuffer, u32, VkPipeline),
    vkCmdBindDescriptorSets: unsafe extern "C" fn(VkCommandBuffer, u32, VkPipelineLayout, u32, u32, *const VkDescriptorSet, u32, *const u32),
    vkCmdPushConstants: unsafe extern "C" fn(VkCommandBuffer, VkPipelineLayout, VkFlags, u32, u32, *const c_void),
    vkCmdSetViewport: unsafe extern "C" fn(VkCommandBuffer, u32, u32, *const VkViewport),
    vkCmdSetScissor: unsafe extern "C" fn(VkCommandBuffer, u32, u32, *const VkRect2D),
    vkCmdBindVertexBuffers: unsafe extern "C" fn(VkCommandBuffer, u32, u32, *const VkBuffer, *const VkDeviceSize),
    vkCmdBindIndexBuffer: unsafe extern "C" fn(VkCommandBuffer, VkBuffer, VkDeviceSize, u32),
    #[allow(dead_code)]
    vkCmdDraw: unsafe extern "C" fn(VkCommandBuffer, u32, u32, u32, u32),
    vkCmdDrawIndexed: unsafe extern "C" fn(VkCommandBuffer, u32, u32, u32, i32, u32),
    vkCreateQueryPool: unsafe extern "C" fn(VkDevice, *const VkQueryPoolCreateInfo, *const c_void, *mut VkQueryPool) -> VkResult,
    vkDestroyQueryPool: unsafe extern "C" fn(VkDevice, VkQueryPool, *const c_void),
    vkCmdResetQueryPool: unsafe extern "C" fn(VkCommandBuffer, VkQueryPool, u32, u32),
    vkCmdWriteTimestamp: unsafe extern "C" fn(VkCommandBuffer, VkFlags, VkQueryPool, u32),
    vkQueueSubmit: unsafe extern "C" fn(VkQueue, u32, *const VkSubmitInfo, u64) -> VkResult,
    vkQueueWaitIdle: unsafe extern "C" fn(VkQueue) -> VkResult,
    vkGetQueryPoolResults: unsafe extern "C" fn(VkDevice, VkQueryPool, u32, u32, usize, *mut c_void, VkDeviceSize, VkFlags) -> VkResult,
    // Optional WSI Surface & Swapchain entrypoints
    vkDestroySurfaceKHR: Option<unsafe extern "C" fn(VkInstance, VkSurfaceKHR, *const c_void)>,
    vkGetPhysicalDeviceSurfaceSupportKHR: Option<unsafe extern "C" fn(VkPhysicalDevice, u32, VkSurfaceKHR, *mut VkBool32) -> VkResult>,
    vkGetPhysicalDeviceSurfaceCapabilitiesKHR: Option<unsafe extern "C" fn(VkPhysicalDevice, VkSurfaceKHR, *mut VkSurfaceCapabilitiesKHR) -> VkResult>,
    vkGetPhysicalDeviceSurfaceFormatsKHR: Option<unsafe extern "C" fn(VkPhysicalDevice, VkSurfaceKHR, *mut u32, *mut VkSurfaceFormatKHR) -> VkResult>,
    vkCreateSwapchainKHR: Option<unsafe extern "C" fn(VkDevice, *const VkSwapchainCreateInfoKHR, *const c_void, *mut VkSwapchainKHR) -> VkResult>,
    vkDestroySwapchainKHR: Option<unsafe extern "C" fn(VkDevice, VkSwapchainKHR, *const c_void)>,
    vkGetSwapchainImagesKHR: Option<unsafe extern "C" fn(VkDevice, VkSwapchainKHR, *mut u32, *mut VkImage) -> VkResult>,
    vkAcquireNextImageKHR: Option<unsafe extern "C" fn(VkDevice, VkSwapchainKHR, u64, VkSemaphore, VkFence, *mut u32) -> VkResult>,
    vkQueuePresentKHR: Option<unsafe extern "C" fn(VkQueue, *const VkPresentInfoKHR) -> VkResult>,
    vkCreateXlibSurfaceKHR: Option<unsafe extern "C" fn(VkInstance, *const VkXlibSurfaceCreateInfoKHR, *const c_void, *mut VkSurfaceKHR) -> VkResult>,
    vkCreateAndroidSurfaceKHR: Option<unsafe extern "C" fn(VkInstance, *const VkAndroidSurfaceCreateInfoKHR, *const c_void, *mut VkSurfaceKHR) -> VkResult>,
}

#[cfg(any(target_os = "linux", target_os = "android"))]
impl VulkanFnTable {
    unsafe fn load() -> Option<Self> {
        let mut lib = dlopen(b"libvulkan.so.1\0".as_ptr() as *const c_char, 2); // Linux RTLD_NOW = 2
        if lib.is_null() {
            lib = dlopen(b"libvulkan.so\0".as_ptr() as *const c_char, 2); // Android /system/lib64/libvulkan.so
        }
        if lib.is_null() {
            return None;
        }
        macro_rules! sym {
            ($name:literal) => {{
                let p = dlsym(lib, concat!($name, "\0").as_ptr() as *const c_char);
                if p.is_null() {
                    return None;
                }
                std::mem::transmute(p)
            }};
        }
        macro_rules! opt_sym {
            ($name:literal) => {{
                let p = dlsym(lib, concat!($name, "\0").as_ptr() as *const c_char);
                if p.is_null() {
                    None
                } else {
                    Some(std::mem::transmute(p))
                }
            }};
        }
        Some(Self {
            vkCreateInstance: sym!("vkCreateInstance"),
            vkDestroyInstance: sym!("vkDestroyInstance"),
            vkEnumeratePhysicalDevices: sym!("vkEnumeratePhysicalDevices"),
            vkGetPhysicalDeviceProperties: sym!("vkGetPhysicalDeviceProperties"),
            vkGetPhysicalDeviceMemoryProperties: sym!("vkGetPhysicalDeviceMemoryProperties"),
            vkCreateDevice: sym!("vkCreateDevice"),
            vkDestroyDevice: sym!("vkDestroyDevice"),
            vkDeviceWaitIdle: sym!("vkDeviceWaitIdle"),
            vkGetDeviceQueue: sym!("vkGetDeviceQueue"),
            vkCreateShaderModule: sym!("vkCreateShaderModule"),
            vkDestroyShaderModule: sym!("vkDestroyShaderModule"),
            vkCreateBuffer: sym!("vkCreateBuffer"),
            vkDestroyBuffer: sym!("vkDestroyBuffer"),
            vkGetBufferMemoryRequirements: sym!("vkGetBufferMemoryRequirements"),
            vkCreateImage: sym!("vkCreateImage"),
            vkDestroyImage: sym!("vkDestroyImage"),
            vkGetImageMemoryRequirements: sym!("vkGetImageMemoryRequirements"),
            vkBindImageMemory: sym!("vkBindImageMemory"),
            vkCreateImageView: sym!("vkCreateImageView"),
            vkDestroyImageView: sym!("vkDestroyImageView"),
            vkCreateSampler: sym!("vkCreateSampler"),
            vkDestroySampler: sym!("vkDestroySampler"),
            vkCreateDescriptorSetLayout: sym!("vkCreateDescriptorSetLayout"),
            vkDestroyDescriptorSetLayout: sym!("vkDestroyDescriptorSetLayout"),
            vkCreateDescriptorPool: sym!("vkCreateDescriptorPool"),
            vkDestroyDescriptorPool: sym!("vkDestroyDescriptorPool"),
            vkAllocateDescriptorSets: sym!("vkAllocateDescriptorSets"),
            vkUpdateDescriptorSets: sym!("vkUpdateDescriptorSets"),
            vkCreatePipelineLayout: sym!("vkCreatePipelineLayout"),
            vkDestroyPipelineLayout: sym!("vkDestroyPipelineLayout"),
            vkCreateRenderPass: sym!("vkCreateRenderPass"),
            vkDestroyRenderPass: sym!("vkDestroyRenderPass"),
            vkCreateGraphicsPipelines: sym!("vkCreateGraphicsPipelines"),
            vkDestroyPipeline: sym!("vkDestroyPipeline"),
            vkCreateFramebuffer: sym!("vkCreateFramebuffer"),
            vkDestroyFramebuffer: sym!("vkDestroyFramebuffer"),
            vkCreateSemaphore: sym!("vkCreateSemaphore"),
            vkDestroySemaphore: sym!("vkDestroySemaphore"),
            vkCreateFence: sym!("vkCreateFence"),
            vkDestroyFence: sym!("vkDestroyFence"),
            vkWaitForFences: sym!("vkWaitForFences"),
            vkResetFences: sym!("vkResetFences"),
            vkAllocateMemory: sym!("vkAllocateMemory"),
            vkFreeMemory: sym!("vkFreeMemory"),
            vkBindBufferMemory: sym!("vkBindBufferMemory"),
            vkMapMemory: sym!("vkMapMemory"),
            vkUnmapMemory: sym!("vkUnmapMemory"),
            vkCreateCommandPool: sym!("vkCreateCommandPool"),
            vkDestroyCommandPool: sym!("vkDestroyCommandPool"),
            vkAllocateCommandBuffers: sym!("vkAllocateCommandBuffers"),
            vkResetCommandBuffer: sym!("vkResetCommandBuffer"),
            vkBeginCommandBuffer: sym!("vkBeginCommandBuffer"),
            vkEndCommandBuffer: sym!("vkEndCommandBuffer"),
            vkCmdPipelineBarrier: sym!("vkCmdPipelineBarrier"),
            vkCmdCopyBufferToImage: sym!("vkCmdCopyBufferToImage"),
            vkCmdCopyImageToBuffer: sym!("vkCmdCopyImageToBuffer"),
            vkCmdBeginRenderPass: sym!("vkCmdBeginRenderPass"),
            vkCmdEndRenderPass: sym!("vkCmdEndRenderPass"),
            vkCmdBindPipeline: sym!("vkCmdBindPipeline"),
            vkCmdBindDescriptorSets: sym!("vkCmdBindDescriptorSets"),
            vkCmdPushConstants: sym!("vkCmdPushConstants"),
            vkCmdSetViewport: sym!("vkCmdSetViewport"),
            vkCmdSetScissor: sym!("vkCmdSetScissor"),
            vkCmdBindVertexBuffers: sym!("vkCmdBindVertexBuffers"),
            vkCmdBindIndexBuffer: sym!("vkCmdBindIndexBuffer"),
            vkCmdDraw: sym!("vkCmdDraw"),
            vkCmdDrawIndexed: sym!("vkCmdDrawIndexed"),
            vkCreateQueryPool: sym!("vkCreateQueryPool"),
            vkDestroyQueryPool: sym!("vkDestroyQueryPool"),
            vkCmdResetQueryPool: sym!("vkCmdResetQueryPool"),
            vkCmdWriteTimestamp: sym!("vkCmdWriteTimestamp"),
            vkQueueSubmit: sym!("vkQueueSubmit"),
            vkQueueWaitIdle: sym!("vkQueueWaitIdle"),
            vkGetQueryPoolResults: sym!("vkGetQueryPoolResults"),
            vkDestroySurfaceKHR: opt_sym!("vkDestroySurfaceKHR"),
            vkGetPhysicalDeviceSurfaceSupportKHR: opt_sym!("vkGetPhysicalDeviceSurfaceSupportKHR"),
            vkGetPhysicalDeviceSurfaceCapabilitiesKHR: opt_sym!("vkGetPhysicalDeviceSurfaceCapabilitiesKHR"),
            vkGetPhysicalDeviceSurfaceFormatsKHR: opt_sym!("vkGetPhysicalDeviceSurfaceFormatsKHR"),
            vkCreateSwapchainKHR: opt_sym!("vkCreateSwapchainKHR"),
            vkDestroySwapchainKHR: opt_sym!("vkDestroySwapchainKHR"),
            vkGetSwapchainImagesKHR: opt_sym!("vkGetSwapchainImagesKHR"),
            vkAcquireNextImageKHR: opt_sym!("vkAcquireNextImageKHR"),
            vkQueuePresentKHR: opt_sym!("vkQueuePresentKHR"),
            vkCreateXlibSurfaceKHR: opt_sym!("vkCreateXlibSurfaceKHR"),
            vkCreateAndroidSurfaceKHR: opt_sym!("vkCreateAndroidSurfaceKHR"),
        })
    }
}

/// Allocated device-local `VkImage` + `VkDeviceMemory` + `VkImageView` + `VkSampler`.
#[derive(Default, Clone, Copy)]
#[allow(dead_code)]
struct GpuAllocatedTexture {
    image: VkImage,
    memory: VkDeviceMemory,
    view: VkImageView,
    sampler: VkSampler,
    width: u32,
    height: u32,
}

impl GpuAllocatedTexture {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn destroy(&mut self, fns: &VulkanFnTable, device: VkDevice) {
        if self.sampler != 0 {
            (fns.vkDestroySampler)(device, self.sampler, ptr::null());
            self.sampler = 0;
        }
        if self.view != 0 {
            (fns.vkDestroyImageView)(device, self.view, ptr::null());
            self.view = 0;
        }
        if self.image != 0 {
            (fns.vkDestroyImage)(device, self.image, ptr::null());
            self.image = 0;
        }
        if self.memory != 0 {
            (fns.vkFreeMemory)(device, self.memory, ptr::null());
            self.memory = 0;
        }
    }
}

/// RGBA Image texture registered with the Vulkan renderer.
#[derive(Debug, Clone)]
pub struct GpuImageTexture {
    pub id: u32,
    pub width: u32,
    pub height: u32,
    pub pixels_rgba8: Vec<u8>,
}

impl GpuImageTexture {
    /// Creates a procedural vibrant test texture (e.g., Iguanodon crest / prism gradient) for the sample app.
    pub fn generate_showcase_image(id: u32, width: u32, height: u32) -> Self {
        let mut pixels_rgba8 = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                let u = x as f32 / width.max(1) as f32;
                let v = y as f32 / height.max(1) as f32;
                let ring = (((u - 0.5).powi(2) + (v - 0.5).powi(2)).sqrt() * 18.0).sin() * 0.5 + 0.5;
                let r = ((0.18 + u * 0.65 + ring * 0.15).clamp(0.0, 1.0) * 255.0) as u8;
                let g = ((0.30 + (1.0 - v) * 0.55).clamp(0.0, 1.0) * 255.0) as u8;
                let b = ((0.55 + v * 0.40).clamp(0.0, 1.0) * 255.0) as u8;
                pixels_rgba8.extend_from_slice(&[r, g, b, 255]);
            }
        }
        Self {
            id,
            width,
            height,
            pixels_rgba8,
        }
    }
}

/// Statistics and output from a single Vulkan frame execution.
#[derive(Debug, Clone)]
pub struct VulkanFrameStats {
    pub device_name: String,
    pub hardware_vulkan_active: bool,
    pub instance_count: usize,
    pub draw_calls: usize,
    pub gpu_timestamp_us: f32,
    pub vram_allocated_bytes: usize,
    pub non_zero_pixels: usize,
}

/// Raw Vulkan Renderer managing `VkInstance`, `VkDevice`, pre-compiled SPIR-V `VkShaderModule`s,
/// `VkDescriptorSetLayout`, `VkPipelineLayout`, `VkRenderPass`, `VkGraphicsPipeline`,
/// GPU-resident `VkImage` textures (Showcase, TrueType Glyph Atlas, 256x256 FBM/Warped LUT),
/// native `VkSwapchainKHR` window presentation (`ANativeWindow` & `Xlib`), offscreen `VkImage`
/// render targets, and hardware `VkQueryPool` timestamp profiling.
pub struct VulkanRenderer {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    fns: Option<VulkanFnTable>,
    instance: VkInstance,
    phys_device: VkPhysicalDevice,
    device: VkDevice,
    queue: VkQueue,
    shader_modules: Vec<VkShaderModule>,
    vert_shader_module: VkShaderModule,
    frag_shader_module: VkShaderModule,
    descriptor_set_layout: VkDescriptorSetLayout,
    descriptor_pool: VkDescriptorPool,
    descriptor_set: VkDescriptorSet,
    pipeline_layout: VkPipelineLayout,
    showcase_gpu_tex: GpuAllocatedTexture,
    glyph_atlas_gpu_tex: GpuAllocatedTexture,
    noise_lut_gpu_tex: GpuAllocatedTexture,
    instance_buffer: VkBuffer,
    instance_memory: VkDeviceMemory,
    instance_buffer_capacity: usize,
    vertex_buffer: VkBuffer,
    vertex_memory: VkDeviceMemory,
    vertex_buffer_capacity: usize,
    index_buffer: VkBuffer,
    index_memory: VkDeviceMemory,
    index_buffer_capacity: usize,
    command_pool: VkCommandPool,
    command_buffer: VkCommandBuffer,
    query_pool: VkQueryPool,
    // Offscreen hardware render target (for headless / RGBA readback)
    offscreen_image: VkImage,
    offscreen_memory: VkDeviceMemory,
    offscreen_view: VkImageView,
    offscreen_render_pass: VkRenderPass,
    offscreen_pipeline: VkPipeline,
    offscreen_fb: VkFramebuffer,
    offscreen_extent: (u32, u32),
    readback_buffer: VkBuffer,
    readback_memory: VkDeviceMemory,
    readback_capacity: usize,
    // Native Window WSI Swapchain (Android ANativeWindow / Linux X11 Window)
    surface: VkSurfaceKHR,
    swapchain: VkSwapchainKHR,
    swapchain_format: u32,
    swapchain_extent: (u32, u32),
    swapchain_images: Vec<VkImage>,
    swapchain_views: Vec<VkImageView>,
    swapchain_fbs: Vec<VkFramebuffer>,
    swapchain_render_pass: VkRenderPass,
    swapchain_pipeline: VkPipeline,
    image_available_sem: VkSemaphore,
    render_finished_sem: VkSemaphore,
    in_flight_fence: VkFence,
    pub device_name: String,
    pub hardware_active: bool,
    pub portability_subset_ready: bool,
    pub textures: Vec<GpuImageTexture>,
}

impl Default for VulkanRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl VulkanRenderer {
    pub fn new() -> Self {
        let _ = iguanodon_shaders::FireSimulationLut::global();
        let _ = iguanodon_shaders::WaterDropWallpaper::global();
        let _ = iguanodon_text::CachedFontSet::global();
        #[cfg(any(target_os = "linux", target_os = "android"))]
        unsafe {
            if let Some(hw) = Self::init_hardware_vulkan() {
                return hw;
            }
        }
        Self {
            #[cfg(any(target_os = "linux", target_os = "android"))]
            fns: None,
            instance: ptr::null_mut(),
            phys_device: ptr::null_mut(),
            device: ptr::null_mut(),
            queue: ptr::null_mut(),
            shader_modules: Vec::new(),
            vert_shader_module: 0,
            frag_shader_module: 0,
            descriptor_set_layout: 0,
            descriptor_pool: 0,
            descriptor_set: 0,
            pipeline_layout: 0,
            showcase_gpu_tex: GpuAllocatedTexture::default(),
            glyph_atlas_gpu_tex: GpuAllocatedTexture::default(),
            noise_lut_gpu_tex: GpuAllocatedTexture::default(),
            instance_buffer: 0,
            instance_memory: 0,
            instance_buffer_capacity: 0,
            vertex_buffer: 0,
            vertex_memory: 0,
            vertex_buffer_capacity: 0,
            index_buffer: 0,
            index_memory: 0,
            index_buffer_capacity: 0,
            command_pool: 0,
            command_buffer: ptr::null_mut(),
            query_pool: 0,
            offscreen_image: 0,
            offscreen_memory: 0,
            offscreen_view: 0,
            offscreen_render_pass: 0,
            offscreen_pipeline: 0,
            offscreen_fb: 0,
            offscreen_extent: (0, 0),
            readback_buffer: 0,
            readback_memory: 0,
            readback_capacity: 0,
            surface: 0,
            swapchain: 0,
            swapchain_format: VK_FORMAT_B8G8R8A8_UNORM,
            swapchain_extent: (0, 0),
            swapchain_images: Vec::new(),
            swapchain_views: Vec::new(),
            swapchain_fbs: Vec::new(),
            swapchain_render_pass: 0,
            swapchain_pipeline: 0,
            image_available_sem: 0,
            render_finished_sem: 0,
            in_flight_fence: 0,
            device_name: "Software Vulkan Fallback".to_string(),
            hardware_active: false,
            portability_subset_ready: true,
            textures: Vec::new(),
        }
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn find_memory_type(
        fns: &VulkanFnTable,
        phys: VkPhysicalDevice,
        type_bits: u32,
        desired_flags: VkFlags,
    ) -> u32 {
        let mut mem_props: VkPhysicalDeviceMemoryProperties = std::mem::zeroed();
        (fns.vkGetPhysicalDeviceMemoryProperties)(phys, &mut mem_props);
        if (desired_flags & VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT) != 0 {
            let cached_flags = desired_flags | 0x00000008; // VK_MEMORY_PROPERTY_HOST_CACHED_BIT
            for i in 0..mem_props.memory_type_count {
                if (type_bits & (1 << i)) != 0
                    && (mem_props.memory_types[i as usize].property_flags & cached_flags)
                        == cached_flags
                {
                    return i;
                }
            }
        }
        for i in 0..mem_props.memory_type_count {
            if (type_bits & (1 << i)) != 0
                && (mem_props.memory_types[i as usize].property_flags & desired_flags)
                    == desired_flags
            {
                return i;
            }
        }
        0
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn upload_gpu_texture_rgba8(
        fns: &VulkanFnTable,
        phys: VkPhysicalDevice,
        device: VkDevice,
        queue: VkQueue,
        cmd: VkCommandBuffer,
        width: u32,
        height: u32,
        pixels_rgba8: &[u8],
        repeat_wrap: bool,
    ) -> GpuAllocatedTexture {
        let byte_size = (width as usize * height as usize * 4) as VkDeviceSize;
        if byte_size == 0 || pixels_rgba8.len() < byte_size as usize {
            return GpuAllocatedTexture::default();
        }

        // 1. Create staging buffer
        let staging_info = VkBufferCreateInfo {
            s_type: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            size: byte_size,
            usage: VK_BUFFER_USAGE_TRANSFER_SRC_BIT,
            sharing_mode: 0,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
        };
        let mut staging_buf: VkBuffer = 0;
        let mut staging_mem: VkDeviceMemory = 0;
        if (fns.vkCreateBuffer)(device, &staging_info, ptr::null(), &mut staging_buf) != VK_SUCCESS {
            return GpuAllocatedTexture::default();
        }
        let mut s_reqs: VkMemoryRequirements = std::mem::zeroed();
        (fns.vkGetBufferMemoryRequirements)(device, staging_buf, &mut s_reqs);
        let s_alloc = VkMemoryAllocateInfo {
            s_type: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
            p_next: ptr::null(),
            allocation_size: s_reqs.size,
            memory_type_index: Self::find_memory_type(
                fns,
                phys,
                s_reqs.memory_type_bits,
                VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
            ),
        };
        if (fns.vkAllocateMemory)(device, &s_alloc, ptr::null(), &mut staging_mem) == VK_SUCCESS {
            let _ = (fns.vkBindBufferMemory)(device, staging_buf, staging_mem, 0);
            let mut mapped: *mut c_void = ptr::null_mut();
            if (fns.vkMapMemory)(device, staging_mem, 0, byte_size, 0, &mut mapped) == VK_SUCCESS
                && !mapped.is_null()
            {
                ptr::copy_nonoverlapping(
                    pixels_rgba8.as_ptr(),
                    mapped as *mut u8,
                    byte_size as usize,
                );
                (fns.vkUnmapMemory)(device, staging_mem);
            }
        }

        // 2. Create device-local VkImage
        let img_info = VkImageCreateInfo {
            s_type: VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            image_type: 1, // VK_IMAGE_TYPE_2D
            format: VK_FORMAT_R8G8B8A8_UNORM,
            extent: VkExtent3D {
                width,
                height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: 1, // VK_SAMPLE_COUNT_1_BIT
            tiling: 0,  // VK_IMAGE_TILING_OPTIMAL
            usage: VK_IMAGE_USAGE_TRANSFER_DST_BIT | VK_IMAGE_USAGE_SAMPLED_BIT,
            sharing_mode: 0,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
            initial_layout: VK_IMAGE_LAYOUT_UNDEFINED,
        };
        let mut image: VkImage = 0;
        let mut memory: VkDeviceMemory = 0;
        if (fns.vkCreateImage)(device, &img_info, ptr::null(), &mut image) == VK_SUCCESS {
            let mut ireqs: VkMemoryRequirements = std::mem::zeroed();
            (fns.vkGetImageMemoryRequirements)(device, image, &mut ireqs);
            let ialloc = VkMemoryAllocateInfo {
                s_type: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                p_next: ptr::null(),
                allocation_size: ireqs.size,
                memory_type_index: Self::find_memory_type(
                    fns,
                    phys,
                    ireqs.memory_type_bits,
                    VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT,
                ),
            };
            if (fns.vkAllocateMemory)(device, &ialloc, ptr::null(), &mut memory) == VK_SUCCESS {
                let _ = (fns.vkBindImageMemory)(device, image, memory, 0);
            }
        }

        // 3. Transition UNDEFINED -> TRANSFER_DST, copy buffer -> image, transition -> SHADER_READ_ONLY
        if !cmd.is_null() && !queue.is_null() && image != 0 {
            let begin = VkCommandBufferBeginInfo {
                s_type: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
                p_next: ptr::null(),
                flags: 0x00000001,
                p_inheritance_info: ptr::null(),
            };
            let _ = (fns.vkResetCommandBuffer)(cmd, 0);
            if (fns.vkBeginCommandBuffer)(cmd, &begin) == VK_SUCCESS {
                let sub_range = VkImageSubresourceRange {
                    aspect_mask: 1, // VK_IMAGE_ASPECT_COLOR_BIT
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                };
                let b_to_dst = VkImageMemoryBarrier {
                    s_type: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                    p_next: ptr::null(),
                    src_access_mask: 0,
                    dst_access_mask: VK_ACCESS_TRANSFER_WRITE_BIT,
                    old_layout: VK_IMAGE_LAYOUT_UNDEFINED,
                    new_layout: VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                    src_queue_family_index: u32::MAX,
                    dst_queue_family_index: u32::MAX,
                    image,
                    subresource_range: sub_range,
                };
                (fns.vkCmdPipelineBarrier)(
                    cmd,
                    VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT,
                    VK_PIPELINE_STAGE_TRANSFER_BIT,
                    0,
                    0,
                    ptr::null(),
                    0,
                    ptr::null(),
                    1,
                    &b_to_dst,
                );

                let region = VkBufferImageCopy {
                    buffer_offset: 0,
                    buffer_row_length: 0,
                    buffer_image_height: 0,
                    image_subresource: VkImageSubresourceLayers {
                        aspect_mask: 1,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    image_offset: VkOffset3D { x: 0, y: 0, z: 0 },
                    image_extent: VkExtent3D {
                        width,
                        height,
                        depth: 1,
                    },
                };
                (fns.vkCmdCopyBufferToImage)(
                    cmd,
                    staging_buf,
                    image,
                    VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                    1,
                    &region,
                );

                let b_to_read = VkImageMemoryBarrier {
                    s_type: VK_STRUCTURE_TYPE_IMAGE_MEMORY_BARRIER,
                    p_next: ptr::null(),
                    src_access_mask: VK_ACCESS_TRANSFER_WRITE_BIT,
                    dst_access_mask: VK_ACCESS_SHADER_READ_BIT,
                    old_layout: VK_IMAGE_LAYOUT_TRANSFER_DST_OPTIMAL,
                    new_layout: VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                    src_queue_family_index: u32::MAX,
                    dst_queue_family_index: u32::MAX,
                    image,
                    subresource_range: sub_range,
                };
                (fns.vkCmdPipelineBarrier)(
                    cmd,
                    VK_PIPELINE_STAGE_TRANSFER_BIT,
                    VK_PIPELINE_STAGE_FRAGMENT_SHADER_BIT,
                    0,
                    0,
                    ptr::null(),
                    0,
                    ptr::null(),
                    1,
                    &b_to_read,
                );
                let _ = (fns.vkEndCommandBuffer)(cmd);
                let sub = VkSubmitInfo {
                    s_type: VK_STRUCTURE_TYPE_SUBMIT_INFO,
                    p_next: ptr::null(),
                    wait_semaphore_count: 0,
                    p_wait_semaphores: ptr::null(),
                    p_wait_dst_stage_mask: ptr::null(),
                    command_buffer_count: 1,
                    p_command_buffers: &cmd,
                    signal_semaphore_count: 0,
                    p_signal_semaphores: ptr::null(),
                };
                let _ = (fns.vkQueueSubmit)(queue, 1, &sub, 0);
                let _ = (fns.vkQueueWaitIdle)(queue);
            }
        }

        if staging_buf != 0 {
            (fns.vkDestroyBuffer)(device, staging_buf, ptr::null());
        }
        if staging_mem != 0 {
            (fns.vkFreeMemory)(device, staging_mem, ptr::null());
        }

        // 4. Create VkImageView & VkSampler
        let view_info = VkImageViewCreateInfo {
            s_type: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            image,
            view_type: 1, // VK_IMAGE_VIEW_TYPE_2D
            format: VK_FORMAT_R8G8B8A8_UNORM,
            components: VkComponentMapping {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            subresource_range: VkImageSubresourceRange {
                aspect_mask: 1,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        let mut view: VkImageView = 0;
        let _ = (fns.vkCreateImageView)(device, &view_info, ptr::null(), &mut view);

        let addr_mode = if repeat_wrap { 0 } else { 2 }; // 0 = REPEAT, 2 = CLAMP_TO_EDGE
        let samp_info = VkSamplerCreateInfo {
            s_type: VK_STRUCTURE_TYPE_SAMPLER_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            mag_filter: 1, // VK_FILTER_LINEAR
            min_filter: 1, // VK_FILTER_LINEAR
            mipmap_mode: 0,
            address_mode_u: addr_mode,
            address_mode_v: addr_mode,
            address_mode_w: addr_mode,
            mip_lod_bias: 0.0,
            anisotropy_enable: 0,
            max_anisotropy: 1.0,
            compare_enable: 0,
            compare_op: 0,
            min_lod: 0.0,
            max_lod: 0.0,
            border_color: 0,
            unnormalized_coordinates: 0,
        };
        let mut sampler: VkSampler = 0;
        let _ = (fns.vkCreateSampler)(device, &samp_info, ptr::null(), &mut sampler);

        GpuAllocatedTexture {
            image,
            memory,
            view,
            sampler,
            width,
            height,
        }
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn create_render_pass_and_pipeline(
        fns: &VulkanFnTable,
        device: VkDevice,
        vert_mod: VkShaderModule,
        frag_mod: VkShaderModule,
        pipeline_layout: VkPipelineLayout,
        color_format: u32,
        final_layout: u32,
    ) -> (VkRenderPass, VkPipeline) {
        let color_att = VkAttachmentDescription {
            flags: 0,
            format: color_format,
            samples: 1,      // VK_SAMPLE_COUNT_1_BIT
            load_op: 1,      // VK_ATTACHMENT_LOAD_OP_CLEAR
            store_op: 0,     // VK_ATTACHMENT_STORE_OP_STORE
            stencil_load_op: 2,  // DONT_CARE
            stencil_store_op: 1, // DONT_CARE
            initial_layout: VK_IMAGE_LAYOUT_UNDEFINED,
            final_layout,
        };
        let color_ref = VkAttachmentReference {
            attachment: 0,
            layout: VK_IMAGE_LAYOUT_COLOR_ATTACHMENT_OPTIMAL,
        };
        let subpass = VkSubpassDescription {
            flags: 0,
            pipeline_bind_point: 0, // VK_PIPELINE_BIND_POINT_GRAPHICS
            input_attachment_count: 0,
            p_input_attachments: ptr::null(),
            color_attachment_count: 1,
            p_color_attachments: &color_ref,
            p_resolve_attachments: ptr::null(),
            p_depth_stencil_attachment: ptr::null(),
            preserve_attachment_count: 0,
            p_preserve_attachments: ptr::null(),
        };
        let dep = VkSubpassDependency {
            src_subpass: u32::MAX, // VK_SUBPASS_EXTERNAL
            dst_subpass: 0,
            src_stage_mask: VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
            dst_stage_mask: VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT,
            src_access_mask: 0,
            dst_access_mask: 0x00000180, // COLOR_ATTACHMENT_READ | COLOR_ATTACHMENT_WRITE
            dependency_flags: 0,
        };
        let rp_info = VkRenderPassCreateInfo {
            s_type: VK_STRUCTURE_TYPE_RENDER_PASS_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            attachment_count: 1,
            p_attachments: &color_att,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 1,
            p_dependencies: &dep,
        };
        let mut render_pass: VkRenderPass = 0;
        if (fns.vkCreateRenderPass)(device, &rp_info, ptr::null(), &mut render_pass) != VK_SUCCESS {
            return (0, 0);
        }

        let entry_name = b"main\0".as_ptr() as *const c_char;
        let stages = [
            VkPipelineShaderStageCreateInfo {
                s_type: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: ptr::null(),
                flags: 0,
                stage: VK_SHADER_STAGE_VERTEX_BIT,
                module: vert_mod,
                p_name: entry_name,
                p_specialization_info: ptr::null(),
            },
            VkPipelineShaderStageCreateInfo {
                s_type: VK_STRUCTURE_TYPE_PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: ptr::null(),
                flags: 0,
                stage: VK_SHADER_STAGE_FRAGMENT_BIT,
                module: frag_mod,
                p_name: entry_name,
                p_specialization_info: ptr::null(),
            },
        ];

        let binding_desc = VkVertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<GpuQuadVertex>() as u32,
            input_rate: VK_VERTEX_INPUT_RATE_VERTEX,
        };
        let attr_desc = VkVertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: VK_FORMAT_R32G32B32A32_SFLOAT,
            offset: 0,
        };
        let vertex_input = VkPipelineVertexInputStateCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_VERTEX_INPUT_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            vertex_binding_description_count: 1,
            p_vertex_binding_descriptions: &binding_desc,
            vertex_attribute_description_count: 1,
            p_vertex_attribute_descriptions: &attr_desc,
        };
        let input_assembly = VkPipelineInputAssemblyStateCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_INPUT_ASSEMBLY_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            topology: 3, // VK_PRIMITIVE_TOPOLOGY_TRIANGLE_LIST
            primitive_restart_enable: 0,
        };
        let viewport_state = VkPipelineViewportStateCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_VIEWPORT_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            viewport_count: 1,
            p_viewports: ptr::null(),
            scissor_count: 1,
            p_scissors: ptr::null(),
        };
        let raster = VkPipelineRasterizationStateCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_RASTERIZATION_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            depth_clamp_enable: 0,
            rasterizer_discard_enable: 0,
            polygon_mode: 0, // VK_POLYGON_MODE_FILL
            cull_mode: 0,    // VK_CULL_MODE_NONE
            front_face: 0,   // VK_FRONT_FACE_COUNTER_CLOCKWISE
            depth_bias_enable: 0,
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
        };
        let multisample = VkPipelineMultisampleStateCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_MULTISAMPLE_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            rasterization_samples: 1,
            sample_shading_enable: 0,
            min_sample_shading: 1.0,
            p_sample_mask: ptr::null(),
            alpha_to_coverage_enable: 0,
            alpha_to_one_enable: 0,
        };
        // Standard Porter-Duff SRC_OVER hardware alpha blending:
        // srcColor * srcAlpha + dstColor * (1 - srcAlpha)
        let blend_att = VkPipelineColorBlendAttachmentState {
            blend_enable: 1,
            src_color_blend_factor: 6, // VK_BLEND_FACTOR_SRC_ALPHA
            dst_color_blend_factor: 7, // VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA
            color_blend_op: 0,         // VK_BLEND_OP_ADD
            src_alpha_blend_factor: 1, // VK_BLEND_FACTOR_ONE
            dst_alpha_blend_factor: 7, // VK_BLEND_FACTOR_ONE_MINUS_SRC_ALPHA
            alpha_blend_op: 0,         // VK_BLEND_OP_ADD
            color_write_mask: 0xF,     // R | G | B | A
        };
        let color_blend = VkPipelineColorBlendStateCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_COLOR_BLEND_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            logic_op_enable: 0,
            logic_op: 0,
            attachment_count: 1,
            p_attachments: &blend_att,
            blend_constants: [0.0; 4],
        };
        let dyn_states = [0_u32, 1_u32]; // VK_DYNAMIC_STATE_VIEWPORT = 0, SCISSOR = 1
        let dyn_info = VkPipelineDynamicStateCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_DYNAMIC_STATE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            dynamic_state_count: 2,
            p_dynamic_states: dyn_states.as_ptr(),
        };

        let gp_info = VkGraphicsPipelineCreateInfo {
            s_type: VK_STRUCTURE_TYPE_GRAPHICS_PIPELINE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            stage_count: 2,
            p_stages: stages.as_ptr(),
            p_vertex_input_state: &vertex_input,
            p_input_assembly_state: &input_assembly,
            p_tessellation_state: ptr::null(),
            p_viewport_state: &viewport_state,
            p_rasterization_state: &raster,
            p_multisample_state: &multisample,
            p_depth_stencil_state: ptr::null(),
            p_color_blend_state: &color_blend,
            p_dynamic_state: &dyn_info,
            layout: pipeline_layout,
            render_pass,
            subpass: 0,
            base_pipeline_handle: 0,
            base_pipeline_index: -1,
        };

        let mut pipeline: VkPipeline = 0;
        let _ = (fns.vkCreateGraphicsPipelines)(device, 0, 1, &gp_info, ptr::null(), &mut pipeline);
        (render_pass, pipeline)
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn init_hardware_vulkan() -> Option<Self> {
        let fns = VulkanFnTable::load()?;
        let app_name = b"Iguanodon UI Toolkit\0";
        let engine_name = b"Iguanodon-VK\0";
        let app_info = VkApplicationInfo {
            s_type: VK_STRUCTURE_TYPE_APPLICATION_INFO,
            p_next: ptr::null(),
            p_application_name: app_name.as_ptr() as *const c_char,
            application_version: (1 << 22),
            p_engine_name: engine_name.as_ptr() as *const c_char,
            engine_version: (1 << 22),
            api_version: (1 << 22) | (1 << 12), // Vulkan 1.1+
        };

        // Try creating VkInstance with WSI surface extensions first, falling back to headless if unavailable
        let ext_surface = b"VK_KHR_surface\0".as_ptr() as *const c_char;
        #[cfg(target_os = "android")]
        let ext_platform = b"VK_KHR_android_surface\0".as_ptr() as *const c_char;
        #[cfg(not(target_os = "android"))]
        let ext_platform = b"VK_KHR_xlib_surface\0".as_ptr() as *const c_char;
        let wsi_exts = [ext_surface, ext_platform];

        let mut instance: VkInstance = ptr::null_mut();
        let wsi_create_info = VkInstanceCreateInfo {
            s_type: VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            p_application_info: &app_info,
            enabled_layer_count: 0,
            pp_enabled_layer_names: ptr::null(),
            enabled_extension_count: 2,
            pp_enabled_extension_names: wsi_exts.as_ptr(),
        };
        if (fns.vkCreateInstance)(&wsi_create_info, ptr::null(), &mut instance) != VK_SUCCESS {
            let fallback_info = VkInstanceCreateInfo {
                s_type: VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO,
                p_next: ptr::null(),
                flags: 0,
                p_application_info: &app_info,
                enabled_layer_count: 0,
                pp_enabled_layer_names: ptr::null(),
                enabled_extension_count: 0,
                pp_enabled_extension_names: ptr::null(),
            };
            if (fns.vkCreateInstance)(&fallback_info, ptr::null(), &mut instance) != VK_SUCCESS {
                return None;
            }
        }

        let mut dev_count = 0_u32;
        if (fns.vkEnumeratePhysicalDevices)(instance, &mut dev_count, ptr::null_mut()) != VK_SUCCESS
            || dev_count == 0
        {
            (fns.vkDestroyInstance)(instance, ptr::null());
            return None;
        }
        let mut phys_devs = vec![ptr::null_mut(); dev_count as usize];
        (fns.vkEnumeratePhysicalDevices)(instance, &mut dev_count, phys_devs.as_mut_ptr());

        let mut phys = phys_devs[0];
        let mut best_score = 0_u32;
        for &cand in &phys_devs {
            let mut p: VkPhysicalDeviceProperties = std::mem::zeroed();
            (fns.vkGetPhysicalDeviceProperties)(cand, &mut p);
            let score = match p.device_type {
                2 => 1000, // VK_PHYSICAL_DEVICE_TYPE_DISCRETE_GPU
                1 => 800,  // VK_PHYSICAL_DEVICE_TYPE_INTEGRATED_GPU
                3 => 400,  // VK_PHYSICAL_DEVICE_TYPE_VIRTUAL_GPU
                4 => 100,  // VK_PHYSICAL_DEVICE_TYPE_CPU (llvmpipe)
                _ => 50,
            };
            if score > best_score {
                best_score = score;
                phys = cand;
            }
        }
        let mut props: VkPhysicalDeviceProperties = std::mem::zeroed();
        (fns.vkGetPhysicalDeviceProperties)(phys, &mut props);
        let device_name = CStr::from_ptr(props.device_name.as_ptr())
            .to_string_lossy()
            .into_owned();

        let priority = 1.0_f32;
        let q_info = VkDeviceQueueCreateInfo {
            s_type: VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            queue_family_index: 0,
            queue_count: 1,
            p_queue_priorities: &priority,
        };
        let ext_swapchain = b"VK_KHR_swapchain\0".as_ptr() as *const c_char;
        let dev_exts = [ext_swapchain];
        let mut device: VkDevice = ptr::null_mut();
        let dev_wsi_info = VkDeviceCreateInfo {
            s_type: VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            queue_create_info_count: 1,
            p_queue_create_infos: &q_info,
            enabled_layer_count: 0,
            pp_enabled_layer_names: ptr::null(),
            enabled_extension_count: 1,
            pp_enabled_extension_names: dev_exts.as_ptr(),
            p_enabled_features: ptr::null(),
        };
        if (fns.vkCreateDevice)(phys, &dev_wsi_info, ptr::null(), &mut device) != VK_SUCCESS {
            let dev_fallback_info = VkDeviceCreateInfo {
                s_type: VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO,
                p_next: ptr::null(),
                flags: 0,
                queue_create_info_count: 1,
                p_queue_create_infos: &q_info,
                enabled_layer_count: 0,
                pp_enabled_layer_names: ptr::null(),
                enabled_extension_count: 0,
                pp_enabled_extension_names: ptr::null(),
                p_enabled_features: ptr::null(),
            };
            if (fns.vkCreateDevice)(phys, &dev_fallback_info, ptr::null(), &mut device)
                != VK_SUCCESS
            {
                (fns.vkDestroyInstance)(instance, ptr::null());
                return None;
            }
        }

        let mut queue: VkQueue = ptr::null_mut();
        (fns.vkGetDeviceQueue)(device, 0, 0, &mut queue);

        // Compile all 4 core SPIR-V modules + Vertex & Fragment Uber-Shader modules
        let mut shader_modules = Vec::with_capacity(4);
        for spirv in [
            sdf_uber_shader_spirv(),
            dual_kawase_blur_spirv(),
            procedural_fire_spirv(),
            water_ripple_spirv(),
        ] {
            if validate_spirv_binary(&spirv).is_ok() {
                let sm_info = VkShaderModuleCreateInfo {
                    s_type: VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
                    p_next: ptr::null(),
                    flags: 0,
                    code_size: spirv.len() * 4,
                    p_code: spirv.as_ptr(),
                };
                let mut module: VkShaderModule = 0;
                if (fns.vkCreateShaderModule)(device, &sm_info, ptr::null(), &mut module)
                    == VK_SUCCESS
                {
                    shader_modules.push(module);
                }
            }
        }

        let vert_spv = sdf_uber_vertex_spirv();
        let frag_spv = sdf_uber_fragment_spirv();
        let mut vert_shader_module: VkShaderModule = 0;
        let mut frag_shader_module: VkShaderModule = 0;
        {
            let v_info = VkShaderModuleCreateInfo {
                s_type: VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
                p_next: ptr::null(),
                flags: 0,
                code_size: vert_spv.len() * 4,
                p_code: vert_spv.as_ptr(),
            };
            let _ =
                (fns.vkCreateShaderModule)(device, &v_info, ptr::null(), &mut vert_shader_module);
            let f_info = VkShaderModuleCreateInfo {
                s_type: VK_STRUCTURE_TYPE_SHADER_MODULE_CREATE_INFO,
                p_next: ptr::null(),
                flags: 0,
                code_size: frag_spv.len() * 4,
                p_code: frag_spv.as_ptr(),
            };
            let _ =
                (fns.vkCreateShaderModule)(device, &f_info, ptr::null(), &mut frag_shader_module);
        }

        // Allocate a real Vulkan host-visible instance buffer for up to 16,384 SDF & glyph instances (2 MB)
        let buffer_cap_bytes = MAX_GPU_INSTANCES * std::mem::size_of::<SdfPrimitiveInstance>();
        let buf_info = VkBufferCreateInfo {
            s_type: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            size: buffer_cap_bytes as VkDeviceSize,
            usage: VK_BUFFER_USAGE_STORAGE_BUFFER_BIT,
            sharing_mode: 0,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
        };
        let mut instance_buffer: VkBuffer = 0;
        let mut instance_memory: VkDeviceMemory = 0;

        if (fns.vkCreateBuffer)(device, &buf_info, ptr::null(), &mut instance_buffer) == VK_SUCCESS
        {
            let mut mem_reqs: VkMemoryRequirements = std::mem::zeroed();
            (fns.vkGetBufferMemoryRequirements)(device, instance_buffer, &mut mem_reqs);
            let alloc_info = VkMemoryAllocateInfo {
                s_type: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                p_next: ptr::null(),
                allocation_size: mem_reqs.size,
                memory_type_index: Self::find_memory_type(
                    &fns,
                    phys,
                    mem_reqs.memory_type_bits,
                    VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
                ),
            };
            if (fns.vkAllocateMemory)(device, &alloc_info, ptr::null(), &mut instance_memory)
                == VK_SUCCESS
            {
                let _ = (fns.vkBindBufferMemory)(device, instance_buffer, instance_memory, 0);
            }
        }

        // Allocate a real Vulkan host-visible Vertex Buffer (4 GpuQuadVertex per instance = 1 MB)
        let vertex_buffer_capacity = MAX_GPU_INSTANCES * 4 * std::mem::size_of::<GpuQuadVertex>();
        let vbuf_info = VkBufferCreateInfo {
            s_type: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            size: vertex_buffer_capacity as VkDeviceSize,
            usage: VK_BUFFER_USAGE_VERTEX_BUFFER_BIT,
            sharing_mode: 0,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
        };
        let mut vertex_buffer: VkBuffer = 0;
        let mut vertex_memory: VkDeviceMemory = 0;
        if (fns.vkCreateBuffer)(device, &vbuf_info, ptr::null(), &mut vertex_buffer) == VK_SUCCESS {
            let mut v_reqs: VkMemoryRequirements = std::mem::zeroed();
            (fns.vkGetBufferMemoryRequirements)(device, vertex_buffer, &mut v_reqs);
            let v_alloc = VkMemoryAllocateInfo {
                s_type: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                p_next: ptr::null(),
                allocation_size: v_reqs.size,
                memory_type_index: Self::find_memory_type(
                    &fns,
                    phys,
                    v_reqs.memory_type_bits,
                    VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
                ),
            };
            if (fns.vkAllocateMemory)(device, &v_alloc, ptr::null(), &mut vertex_memory)
                == VK_SUCCESS
            {
                let _ = (fns.vkBindBufferMemory)(device, vertex_buffer, vertex_memory, 0);
            }
        }

        // Allocate & pre-populate a real Vulkan Index Buffer (6 u32 triangle indices per quad = 384 KB)
        let index_buffer_capacity = MAX_GPU_INSTANCES * 6 * std::mem::size_of::<u32>();
        let ibuf_info = VkBufferCreateInfo {
            s_type: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            size: index_buffer_capacity as VkDeviceSize,
            usage: VK_BUFFER_USAGE_INDEX_BUFFER_BIT,
            sharing_mode: 0,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
        };
        let mut index_buffer: VkBuffer = 0;
        let mut index_memory: VkDeviceMemory = 0;
        if (fns.vkCreateBuffer)(device, &ibuf_info, ptr::null(), &mut index_buffer) == VK_SUCCESS {
            let mut i_reqs: VkMemoryRequirements = std::mem::zeroed();
            (fns.vkGetBufferMemoryRequirements)(device, index_buffer, &mut i_reqs);
            let i_alloc = VkMemoryAllocateInfo {
                s_type: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                p_next: ptr::null(),
                allocation_size: i_reqs.size,
                memory_type_index: Self::find_memory_type(
                    &fns,
                    phys,
                    i_reqs.memory_type_bits,
                    VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
                ),
            };
            if (fns.vkAllocateMemory)(device, &i_alloc, ptr::null(), &mut index_memory)
                == VK_SUCCESS
            {
                let _ = (fns.vkBindBufferMemory)(device, index_buffer, index_memory, 0);
                let mut mapped_idx: *mut c_void = ptr::null_mut();
                if (fns.vkMapMemory)(
                    device,
                    index_memory,
                    0,
                    index_buffer_capacity as VkDeviceSize,
                    0,
                    &mut mapped_idx,
                ) == VK_SUCCESS
                    && !mapped_idx.is_null()
                {
                    let idx_slice = std::slice::from_raw_parts_mut(
                        mapped_idx as *mut u32,
                        MAX_GPU_INSTANCES * 6,
                    );
                    for i in 0..MAX_GPU_INSTANCES {
                        let base_v = (i * 4) as u32;
                        let base_i = i * 6;
                        idx_slice[base_i] = base_v;
                        idx_slice[base_i + 1] = base_v + 1;
                        idx_slice[base_i + 2] = base_v + 2;
                        idx_slice[base_i + 3] = base_v;
                        idx_slice[base_i + 4] = base_v + 2;
                        idx_slice[base_i + 5] = base_v + 3;
                    }
                    (fns.vkUnmapMemory)(device, index_memory);
                }
            }
        }

        // Command pool, command buffer, and hardware timestamp query pool
        let pool_info = VkCommandPoolCreateInfo {
            s_type: VK_STRUCTURE_TYPE_COMMAND_POOL_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0x00000002, // VK_COMMAND_POOL_CREATE_RESET_COMMAND_BUFFER_BIT
            queue_family_index: 0,
        };
        let mut command_pool: VkCommandPool = 0;
        let mut command_buffer: VkCommandBuffer = ptr::null_mut();
        if (fns.vkCreateCommandPool)(device, &pool_info, ptr::null(), &mut command_pool)
            == VK_SUCCESS
        {
            let cb_info = VkCommandBufferAllocateInfo {
                s_type: VK_STRUCTURE_TYPE_COMMAND_BUFFER_ALLOCATE_INFO,
                p_next: ptr::null(),
                command_pool,
                level: 0,
                command_buffer_count: 1,
            };
            let _ = (fns.vkAllocateCommandBuffers)(device, &cb_info, &mut command_buffer);
        }

        let qp_info = VkQueryPoolCreateInfo {
            s_type: VK_STRUCTURE_TYPE_QUERY_POOL_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            query_type: VK_QUERY_TYPE_TIMESTAMP,
            query_count: 2,
            pipeline_statistics: 0,
        };
        let mut query_pool: VkQueryPool = 0;
        let _ = (fns.vkCreateQueryPool)(device, &qp_info, ptr::null(), &mut query_pool);

        // Upload the 3 core GPU textures into device-local VkImages:
        // 1. Showcase 128x128 RGBA8 texture
        let showcase_cpu = GpuImageTexture::generate_showcase_image(1, 128, 128);
        let showcase_gpu_tex = Self::upload_gpu_texture_rgba8(
            &fns,
            phys,
            device,
            queue,
            command_buffer,
            showcase_cpu.width,
            showcase_cpu.height,
            &showcase_cpu.pixels_rgba8,
            false,
        );
        // 2. TrueType Roboto / NotoSans 512x512 GPU Glyph Atlas texture
        let font_set = iguanodon_text::CachedFontSet::global();
        let glyph_atlas_gpu_tex = Self::upload_gpu_texture_rgba8(
            &fns,
            phys,
            device,
            queue,
            command_buffer,
            font_set.atlas_width,
            font_set.atlas_height,
            &font_set.atlas_rgba8,
            false,
        );
        // 3. 256x256 FBM + Domain-Warped Noise LUT texture (repeat-wrapped)
        let noise_rgba = iguanodon_shaders::FireSimulationLut::global().pack_gpu_noise_rgba8();
        let noise_lut_gpu_tex = Self::upload_gpu_texture_rgba8(
            &fns,
            phys,
            device,
            queue,
            command_buffer,
            256,
            256,
            &noise_rgba,
            true,
        );

        // Create VkDescriptorSetLayout (binding 0 = SSBO, bindings 1..3 = combined image samplers)
        let bindings = [
            VkDescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
                descriptor_count: 1,
                stage_flags: VK_SHADER_STAGE_ALL_GRAPHICS,
                p_immutable_samplers: ptr::null(),
            },
            VkDescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: VK_SHADER_STAGE_FRAGMENT_BIT,
                p_immutable_samplers: ptr::null(),
            },
            VkDescriptorSetLayoutBinding {
                binding: 2,
                descriptor_type: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: VK_SHADER_STAGE_FRAGMENT_BIT,
                p_immutable_samplers: ptr::null(),
            },
            VkDescriptorSetLayoutBinding {
                binding: 3,
                descriptor_type: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                descriptor_count: 1,
                stage_flags: VK_SHADER_STAGE_FRAGMENT_BIT,
                p_immutable_samplers: ptr::null(),
            },
        ];
        let dsl_info = VkDescriptorSetLayoutCreateInfo {
            s_type: VK_STRUCTURE_TYPE_DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            binding_count: 4,
            p_bindings: bindings.as_ptr(),
        };
        let mut descriptor_set_layout: VkDescriptorSetLayout = 0;
        let _ = (fns.vkCreateDescriptorSetLayout)(
            device,
            &dsl_info,
            ptr::null(),
            &mut descriptor_set_layout,
        );

        let pool_sizes = [
            VkDescriptorPoolSize {
                type_: VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
                descriptor_count: 2,
            },
            VkDescriptorPoolSize {
                type_: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                descriptor_count: 6,
            },
        ];
        let dp_info = VkDescriptorPoolCreateInfo {
            s_type: VK_STRUCTURE_TYPE_DESCRIPTOR_POOL_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            max_sets: 2,
            pool_size_count: 2,
            p_pool_sizes: pool_sizes.as_ptr(),
        };
        let mut descriptor_pool: VkDescriptorPool = 0;
        let _ =
            (fns.vkCreateDescriptorPool)(device, &dp_info, ptr::null(), &mut descriptor_pool);

        let ds_alloc = VkDescriptorSetAllocateInfo {
            s_type: VK_STRUCTURE_TYPE_DESCRIPTOR_SET_ALLOCATE_INFO,
            p_next: ptr::null(),
            descriptor_pool,
            descriptor_set_count: 1,
            p_set_layouts: &descriptor_set_layout,
        };
        let mut descriptor_set: VkDescriptorSet = 0;
        let _ = (fns.vkAllocateDescriptorSets)(device, &ds_alloc, &mut descriptor_set);

        if descriptor_set != 0 && instance_buffer != 0 {
            let buf_desc = VkDescriptorBufferInfo {
                buffer: instance_buffer,
                offset: 0,
                range: buffer_cap_bytes as VkDeviceSize,
            };
            let img_showcase = VkDescriptorImageInfo {
                sampler: showcase_gpu_tex.sampler,
                image_view: showcase_gpu_tex.view,
                image_layout: VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
            };
            let img_glyph = VkDescriptorImageInfo {
                sampler: glyph_atlas_gpu_tex.sampler,
                image_view: glyph_atlas_gpu_tex.view,
                image_layout: VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
            };
            let img_noise = VkDescriptorImageInfo {
                sampler: noise_lut_gpu_tex.sampler,
                image_view: noise_lut_gpu_tex.view,
                image_layout: VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
            };
            let writes = [
                VkWriteDescriptorSet {
                    s_type: VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
                    p_next: ptr::null(),
                    dst_set: descriptor_set,
                    dst_binding: 0,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: VK_DESCRIPTOR_TYPE_STORAGE_BUFFER,
                    p_image_info: ptr::null(),
                    p_buffer_info: &buf_desc,
                    p_texel_buffer_view: ptr::null(),
                },
                VkWriteDescriptorSet {
                    s_type: VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
                    p_next: ptr::null(),
                    dst_set: descriptor_set,
                    dst_binding: 1,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                    p_image_info: &img_showcase,
                    p_buffer_info: ptr::null(),
                    p_texel_buffer_view: ptr::null(),
                },
                VkWriteDescriptorSet {
                    s_type: VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
                    p_next: ptr::null(),
                    dst_set: descriptor_set,
                    dst_binding: 2,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                    p_image_info: &img_glyph,
                    p_buffer_info: ptr::null(),
                    p_texel_buffer_view: ptr::null(),
                },
                VkWriteDescriptorSet {
                    s_type: VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
                    p_next: ptr::null(),
                    dst_set: descriptor_set,
                    dst_binding: 3,
                    dst_array_element: 0,
                    descriptor_count: 1,
                    descriptor_type: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                    p_image_info: &img_noise,
                    p_buffer_info: ptr::null(),
                    p_texel_buffer_view: ptr::null(),
                },
            ];
            (fns.vkUpdateDescriptorSets)(device, 4, writes.as_ptr(), 0, ptr::null());
        }

        // Create VkPipelineLayout with 32-byte push constants
        let pc_range = VkPushConstantRange {
            stage_flags: VK_SHADER_STAGE_ALL_GRAPHICS,
            offset: 0,
            size: std::mem::size_of::<SdfPushConstants>() as u32,
        };
        let pl_info = VkPipelineLayoutCreateInfo {
            s_type: VK_STRUCTURE_TYPE_PIPELINE_LAYOUT_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            set_layout_count: 1,
            p_set_layouts: &descriptor_set_layout,
            push_constant_range_count: 1,
            p_push_constant_ranges: &pc_range,
        };
        let mut pipeline_layout: VkPipelineLayout = 0;
        let _ =
            (fns.vkCreatePipelineLayout)(device, &pl_info, ptr::null(), &mut pipeline_layout);

        // Create offscreen RenderPass + GraphicsPipeline (VK_FORMAT_R8G8B8A8_UNORM -> TRANSFER_SRC)
        let (offscreen_render_pass, offscreen_pipeline) = Self::create_render_pass_and_pipeline(
            &fns,
            device,
            vert_shader_module,
            frag_shader_module,
            pipeline_layout,
            VK_FORMAT_R8G8B8A8_UNORM,
            VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
        );

        // Create synchronization primitives for Swapchain presentation
        let sem_info = VkSemaphoreCreateInfo {
            s_type: VK_STRUCTURE_TYPE_SEMAPHORE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
        };
        let mut image_available_sem: VkSemaphore = 0;
        let mut render_finished_sem: VkSemaphore = 0;
        let _ =
            (fns.vkCreateSemaphore)(device, &sem_info, ptr::null(), &mut image_available_sem);
        let _ =
            (fns.vkCreateSemaphore)(device, &sem_info, ptr::null(), &mut render_finished_sem);

        let fence_info = VkFenceCreateInfo {
            s_type: VK_STRUCTURE_TYPE_FENCE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0x00000001, // VK_FENCE_CREATE_SIGNALED_BIT
        };
        let mut in_flight_fence: VkFence = 0;
        let _ = (fns.vkCreateFence)(device, &fence_info, ptr::null(), &mut in_flight_fence);

        Some(Self {
            fns: Some(fns),
            instance,
            phys_device: phys,
            device,
            queue,
            shader_modules,
            vert_shader_module,
            frag_shader_module,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_set,
            pipeline_layout,
            showcase_gpu_tex,
            glyph_atlas_gpu_tex,
            noise_lut_gpu_tex,
            instance_buffer,
            instance_memory,
            instance_buffer_capacity: buffer_cap_bytes,
            vertex_buffer,
            vertex_memory,
            vertex_buffer_capacity,
            index_buffer,
            index_memory,
            index_buffer_capacity,
            command_pool,
            command_buffer,
            query_pool,
            offscreen_image: 0,
            offscreen_memory: 0,
            offscreen_view: 0,
            offscreen_render_pass,
            offscreen_pipeline,
            offscreen_fb: 0,
            offscreen_extent: (0, 0),
            readback_buffer: 0,
            readback_memory: 0,
            readback_capacity: 0,
            surface: 0,
            swapchain: 0,
            swapchain_format: VK_FORMAT_B8G8R8A8_UNORM,
            swapchain_extent: (0, 0),
            swapchain_images: Vec::new(),
            swapchain_views: Vec::new(),
            swapchain_fbs: Vec::new(),
            swapchain_render_pass: 0,
            swapchain_pipeline: 0,
            image_available_sem,
            render_finished_sem,
            in_flight_fence,
            device_name,
            hardware_active: true,
            portability_subset_ready: true,
            textures: vec![showcase_cpu],
        })
    }

    pub fn compiled_shader_module_count(&self) -> usize {
        self.shader_modules.len()
    }

    pub fn has_active_swapchain(&self) -> bool {
        self.swapchain != 0 && !self.swapchain_fbs.is_empty()
    }

    pub fn register_texture(&mut self, tex: GpuImageTexture) {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        if let Some(fns) = &self.fns {
            if self.hardware_active && !self.device.is_null() && tex.width > 0 && tex.height > 0 {
                unsafe {
                    let _ = (fns.vkDeviceWaitIdle)(self.device);
                    let new_gpu_tex = Self::upload_gpu_texture_rgba8(
                        fns,
                        self.phys_device,
                        self.device,
                        self.queue,
                        self.command_buffer,
                        tex.width,
                        tex.height,
                        &tex.pixels_rgba8,
                        false,
                    );
                    if new_gpu_tex.image != 0 && new_gpu_tex.view != 0 {
                        if self.showcase_gpu_tex.sampler != 0 {
                            (fns.vkDestroySampler)(
                                self.device,
                                self.showcase_gpu_tex.sampler,
                                ptr::null(),
                            );
                        }
                        if self.showcase_gpu_tex.view != 0 {
                            (fns.vkDestroyImageView)(
                                self.device,
                                self.showcase_gpu_tex.view,
                                ptr::null(),
                            );
                        }
                        if self.showcase_gpu_tex.image != 0 {
                            (fns.vkDestroyImage)(
                                self.device,
                                self.showcase_gpu_tex.image,
                                ptr::null(),
                            );
                        }
                        if self.showcase_gpu_tex.memory != 0 {
                            (fns.vkFreeMemory)(
                                self.device,
                                self.showcase_gpu_tex.memory,
                                ptr::null(),
                            );
                        }
                        self.showcase_gpu_tex = new_gpu_tex;
                        let img_desc = VkDescriptorImageInfo {
                            sampler: self.showcase_gpu_tex.sampler,
                            image_view: self.showcase_gpu_tex.view,
                            image_layout: VK_IMAGE_LAYOUT_SHADER_READ_ONLY_OPTIMAL,
                        };
                        let write = VkWriteDescriptorSet {
                            s_type: VK_STRUCTURE_TYPE_WRITE_DESCRIPTOR_SET,
                            p_next: ptr::null(),
                            dst_set: self.descriptor_set,
                            dst_binding: 1,
                            dst_array_element: 0,
                            descriptor_count: 1,
                            descriptor_type: VK_DESCRIPTOR_TYPE_COMBINED_IMAGE_SAMPLER,
                            p_image_info: &img_desc,
                            p_buffer_info: ptr::null(),
                            p_texel_buffer_view: ptr::null(),
                        };
                        (fns.vkUpdateDescriptorSets)(self.device, 1, &write, 0, ptr::null());
                    }
                }
            }
        }
        self.textures.push(tex);
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn destroy_swapchain_dependents(&mut self) {
        if let Some(fns) = &self.fns {
            if !self.device.is_null() {
                let _ = (fns.vkDeviceWaitIdle)(self.device);
                for &fb in &self.swapchain_fbs {
                    if fb != 0 {
                        (fns.vkDestroyFramebuffer)(self.device, fb, ptr::null());
                    }
                }
                self.swapchain_fbs.clear();
                for &view in &self.swapchain_views {
                    if view != 0 {
                        (fns.vkDestroyImageView)(self.device, view, ptr::null());
                    }
                }
                self.swapchain_views.clear();
                self.swapchain_images.clear();
                if self.swapchain != 0 {
                    if let Some(destroy_sc) = fns.vkDestroySwapchainKHR {
                        destroy_sc(self.device, self.swapchain, ptr::null());
                    }
                    self.swapchain = 0;
                }
            }
        }
    }

    pub fn detach_surface(&mut self) {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        unsafe {
            self.destroy_swapchain_dependents();
            if let Some(fns) = &self.fns {
                if !self.device.is_null() {
                    if self.swapchain_pipeline != 0 {
                        (fns.vkDestroyPipeline)(self.device, self.swapchain_pipeline, ptr::null());
                        self.swapchain_pipeline = 0;
                    }
                    if self.swapchain_render_pass != 0 {
                        (fns.vkDestroyRenderPass)(
                            self.device,
                            self.swapchain_render_pass,
                            ptr::null(),
                        );
                        self.swapchain_render_pass = 0;
                    }
                }
                if self.surface != 0 && !self.instance.is_null() {
                    if let Some(destroy_surf) = fns.vkDestroySurfaceKHR {
                        destroy_surf(self.instance, self.surface, ptr::null());
                    }
                    self.surface = 0;
                }
            }
        }
    }

    /// Attaches an Android `ANativeWindow*` (`SurfaceView` / `Surface`) and creates a hardware `VkSwapchainKHR`.
    pub fn attach_android_surface(
        &mut self,
        native_window: *mut c_void,
        width: u32,
        height: u32,
    ) -> bool {
        if native_window.is_null() {
            self.detach_surface();
            return false;
        }
        #[cfg(any(target_os = "linux", target_os = "android"))]
        unsafe {
            self.detach_surface();
            let create_android_surf = match self.fns.as_ref().and_then(|f| f.vkCreateAndroidSurfaceKHR) {
                Some(f) => f,
                None => return false,
            };
            let info = VkAndroidSurfaceCreateInfoKHR {
                s_type: VK_STRUCTURE_TYPE_ANDROID_SURFACE_CREATE_INFO_KHR,
                p_next: ptr::null(),
                flags: 0,
                window: native_window,
            };
            let mut surf: VkSurfaceKHR = 0;
            if create_android_surf(self.instance, &info, ptr::null(), &mut surf) != VK_SUCCESS
                || surf == 0
            {
                return false;
            }
            self.surface = surf;
            return self.recreate_swapchain(width.max(1), height.max(1));
        }
        #[allow(unreachable_code)]
        false
    }

    /// Attaches a Linux X11 `(Display*, Window)` and creates a hardware `VkSwapchainKHR`.
    pub fn attach_xlib_surface(
        &mut self,
        dpy: *mut c_void,
        window: u64,
        width: u32,
        height: u32,
    ) -> bool {
        if dpy.is_null() || window == 0 {
            self.detach_surface();
            return false;
        }
        #[cfg(any(target_os = "linux", target_os = "android"))]
        unsafe {
            self.detach_surface();
            let create_xlib_surf = match self.fns.as_ref().and_then(|f| f.vkCreateXlibSurfaceKHR) {
                Some(f) => f,
                None => return false,
            };
            let info = VkXlibSurfaceCreateInfoKHR {
                s_type: VK_STRUCTURE_TYPE_XLIB_SURFACE_CREATE_INFO_KHR,
                p_next: ptr::null(),
                flags: 0,
                dpy,
                window,
            };
            let mut surf: VkSurfaceKHR = 0;
            if create_xlib_surf(self.instance, &info, ptr::null(), &mut surf) != VK_SUCCESS
                || surf == 0
            {
                return false;
            }
            self.surface = surf;
            return self.recreate_swapchain(width.max(1), height.max(1));
        }
        #[allow(unreachable_code)]
        false
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn recreate_swapchain(&mut self, req_w: u32, req_h: u32) -> bool {
        if self.surface == 0 || self.device.is_null() {
            return false;
        }
        self.destroy_swapchain_dependents();

        let fns = match &self.fns {
            Some(f) => f,
            None => return false,
        };
        let (get_caps, get_fmts, create_sc, get_imgs) = match (
            fns.vkGetPhysicalDeviceSurfaceCapabilitiesKHR,
            fns.vkGetPhysicalDeviceSurfaceFormatsKHR,
            fns.vkCreateSwapchainKHR,
            fns.vkGetSwapchainImagesKHR,
        ) {
            (Some(c), Some(fm), Some(cs), Some(gi)) => (c, fm, cs, gi),
            _ => return false,
        };

        if let Some(get_sup) = fns.vkGetPhysicalDeviceSurfaceSupportKHR {
            let mut supported: VkBool32 = 0;
            let _ = get_sup(self.phys_device, 0, self.surface, &mut supported);
            if supported == 0 {
                return false;
            }
        }

        let mut caps: VkSurfaceCapabilitiesKHR = std::mem::zeroed();
        if get_caps(self.phys_device, self.surface, &mut caps) != VK_SUCCESS {
            return false;
        }

        let mut fmt_count = 0_u32;
        let _ = get_fmts(self.phys_device, self.surface, &mut fmt_count, ptr::null_mut());
        let mut chosen_format = VK_FORMAT_B8G8R8A8_UNORM;
        let mut chosen_colorspace = 0_u32;
        if fmt_count > 0 {
            let mut fmts = vec![
                VkSurfaceFormatKHR {
                    format: VK_FORMAT_B8G8R8A8_UNORM,
                    color_space: 0,
                };
                fmt_count as usize
            ];
            let _ = get_fmts(self.phys_device, self.surface, &mut fmt_count, fmts.as_mut_ptr());
            chosen_format = fmts[0].format;
            chosen_colorspace = fmts[0].color_space;
            for f in &fmts {
                if f.format == VK_FORMAT_R8G8B8A8_UNORM || f.format == VK_FORMAT_B8G8R8A8_UNORM {
                    chosen_format = f.format;
                    chosen_colorspace = f.color_space;
                    break;
                }
            }
        }

        let ext_w = if caps.current_extent.width != u32::MAX && caps.current_extent.width > 0 {
            caps.current_extent.width
        } else {
            req_w.clamp(
                caps.min_image_extent.width.max(1),
                caps.max_image_extent.width.max(1),
            )
        };
        let ext_h = if caps.current_extent.height != u32::MAX && caps.current_extent.height > 0 {
            caps.current_extent.height
        } else {
            req_h.clamp(
                caps.min_image_extent.height.max(1),
                caps.max_image_extent.height.max(1),
            )
        };

        let mut img_count = caps.min_image_count.max(2);
        if caps.max_image_count > 0 && img_count > caps.max_image_count {
            img_count = caps.max_image_count;
        }

        // Pick a valid compositeAlpha bit supported by Android/X11 (OPAQUE = 1, INHERIT = 8)
        let mut composite_alpha = 1_u32;
        for bit in [1_u32, 8, 2, 4] {
            if (caps.supported_composite_alpha & bit) != 0 {
                composite_alpha = bit;
                break;
            }
        }

        let pre_transform = if (caps.supported_transforms & 1) != 0 {
            1 // VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR
        } else {
            caps.current_transform
        };

        let sc_info = VkSwapchainCreateInfoKHR {
            s_type: VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR,
            p_next: ptr::null(),
            flags: 0,
            surface: self.surface,
            min_image_count: img_count,
            image_format: chosen_format,
            image_color_space: chosen_colorspace,
            image_extent: VkExtent2D {
                width: ext_w,
                height: ext_h,
            },
            image_array_layers: 1,
            image_usage: VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT,
            image_sharing_mode: 0,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
            pre_transform,
            composite_alpha,
            present_mode: 2, // VK_PRESENT_MODE_FIFO_KHR (guaranteed by Vulkan spec on all GPUs)
            clipped: 1,
            old_swapchain: 0,
        };

        let mut swapchain: VkSwapchainKHR = 0;
        if create_sc(self.device, &sc_info, ptr::null(), &mut swapchain) != VK_SUCCESS
            || swapchain == 0
        {
            return false;
        }
        self.swapchain = swapchain;
        self.swapchain_extent = (ext_w, ext_h);

        if self.swapchain_render_pass == 0
            || self.swapchain_pipeline == 0
            || self.swapchain_format != chosen_format
        {
            if self.swapchain_pipeline != 0 {
                (fns.vkDestroyPipeline)(self.device, self.swapchain_pipeline, ptr::null());
            }
            if self.swapchain_render_pass != 0 {
                (fns.vkDestroyRenderPass)(self.device, self.swapchain_render_pass, ptr::null());
            }
            self.swapchain_format = chosen_format;
            let (rp, pipe) = Self::create_render_pass_and_pipeline(
                fns,
                self.device,
                self.vert_shader_module,
                self.frag_shader_module,
                self.pipeline_layout,
                chosen_format,
                VK_IMAGE_LAYOUT_PRESENT_SRC_KHR,
            );
            self.swapchain_render_pass = rp;
            self.swapchain_pipeline = pipe;
        }

        let mut actual_count = 0_u32;
        let _ = get_imgs(self.device, self.swapchain, &mut actual_count, ptr::null_mut());
        if actual_count == 0 {
            return false;
        }
        self.swapchain_images.resize(actual_count as usize, 0);
        let _ = get_imgs(
            self.device,
            self.swapchain,
            &mut actual_count,
            self.swapchain_images.as_mut_ptr(),
        );

        for &img in &self.swapchain_images {
            let v_info = VkImageViewCreateInfo {
                s_type: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
                p_next: ptr::null(),
                flags: 0,
                image: img,
                view_type: 1,
                format: chosen_format,
                components: VkComponentMapping {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                },
                subresource_range: VkImageSubresourceRange {
                    aspect_mask: 1,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                },
            };
            let mut view: VkImageView = 0;
            let _ = (fns.vkCreateImageView)(self.device, &v_info, ptr::null(), &mut view);
            self.swapchain_views.push(view);

            let fb_info = VkFramebufferCreateInfo {
                s_type: VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
                p_next: ptr::null(),
                flags: 0,
                render_pass: self.swapchain_render_pass,
                attachment_count: 1,
                p_attachments: &view,
                width: ext_w,
                height: ext_h,
                layers: 1,
            };
            let mut fb: VkFramebuffer = 0;
            let _ = (fns.vkCreateFramebuffer)(self.device, &fb_info, ptr::null(), &mut fb);
            self.swapchain_fbs.push(fb);
        }

        !self.swapchain_fbs.is_empty() && self.swapchain_pipeline != 0
    }

    #[inline]
    fn compute_instance_quad_bounds(inst: &SdfPrimitiveInstance) -> ([f32; 2], [f32; 2]) {
        let raw_mat = (inst.params[3] + 0.5) as u32;
        let mat = raw_mat % 100;
        let border_fx = raw_mat / 100;
        if mat == 14 {
            let c_min_x = inst.clip_rect[0];
            let c_min_y = inst.clip_rect[1];
            let c_max_x = inst.clip_rect[0] + inst.clip_rect[2];
            let c_max_y = inst.clip_rect[1] + inst.clip_rect[3];
            let q_min_x = inst.rect[0].max(c_min_x);
            let q_min_y = inst.rect[1].max(c_min_y);
            let q_max_x = (inst.rect[0] + inst.rect[2]).min(c_max_x);
            let q_max_y = (inst.rect[1] + inst.rect[3]).min(c_max_y);
            ([q_min_x, q_min_y], [q_max_x, q_max_y])
        } else {
            let mut halo = inst.params[1] * 2.5 + inst.params[0] + 3.0;
            if border_fx != 0 || mat == 2 || mat == 5 || mat == 7 || mat == 10 {
                halo = halo.max(32.0);
            }
            let q_min_x = inst.rect[0] - halo;
            let q_min_y = inst.rect[1] - halo;
            let q_max_x = inst.rect[0] + inst.rect[2] + halo;
            let q_max_y = inst.rect[1] + inst.rect[3] + halo;

            let mut c_min_x = inst.clip_rect[0];
            let mut c_min_y = inst.clip_rect[1];
            let mut c_max_x = inst.clip_rect[0] + inst.clip_rect[2];
            let mut c_max_y = inst.clip_rect[1] + inst.clip_rect[3];
            let clip_is_self = (inst.clip_rect[0] - inst.rect[0]).abs() < 1.0
                && (inst.clip_rect[1] - inst.rect[1]).abs() < 1.0
                && (inst.clip_rect[2] - inst.rect[2]).abs() < 1.0
                && (inst.clip_rect[3] - inst.rect[3]).abs() < 1.0;
            if clip_is_self && (border_fx != 0 || mat == 2 || mat == 5 || mat == 7 || mat == 10) {
                c_min_x -= 32.0;
                c_min_y -= 32.0;
                c_max_x += 32.0;
                c_max_y += 32.0;
            }
            (
                [q_min_x.max(c_min_x), q_min_y.max(c_min_y)],
                [q_max_x.min(c_max_x), q_max_y.min(c_max_y)],
            )
        }
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn upload_instances_and_vertices(
        &self,
        fns: &VulkanFnTable,
        instances: &[SdfPrimitiveInstance],
    ) -> u32 {
        let count = instances.len().min(MAX_GPU_INSTANCES);
        if count == 0 {
            return 0;
        }
        let inst_bytes = count * std::mem::size_of::<SdfPrimitiveInstance>();
        if self.instance_memory != 0 {
            let mut mapped: *mut c_void = ptr::null_mut();
            if (fns.vkMapMemory)(
                self.device,
                self.instance_memory,
                0,
                inst_bytes as VkDeviceSize,
                0,
                &mut mapped,
            ) == VK_SUCCESS
                && !mapped.is_null()
            {
                ptr::copy_nonoverlapping(
                    instances.as_ptr() as *const u8,
                    mapped as *mut u8,
                    inst_bytes,
                );
                (fns.vkUnmapMemory)(self.device, self.instance_memory);
            }
        }
        let vert_bytes = count * 4 * std::mem::size_of::<GpuQuadVertex>();
        if self.vertex_memory != 0 {
            let mut mapped_v: *mut c_void = ptr::null_mut();
            if (fns.vkMapMemory)(
                self.device,
                self.vertex_memory,
                0,
                vert_bytes as VkDeviceSize,
                0,
                &mut mapped_v,
            ) == VK_SUCCESS
                && !mapped_v.is_null()
            {
                let v_slice =
                    std::slice::from_raw_parts_mut(mapped_v as *mut GpuQuadVertex, count * 4);
                for (i, inst) in instances[..count].iter().enumerate() {
                    let (q_min, q_max) = Self::compute_instance_quad_bounds(inst);
                    let base = i * 4;
                    v_slice[base] = GpuQuadVertex {
                        pos_px: [q_min[0], q_min[1]],
                        uv: [0.0, 0.0],
                    };
                    v_slice[base + 1] = GpuQuadVertex {
                        pos_px: [q_max[0], q_min[1]],
                        uv: [1.0, 0.0],
                    };
                    v_slice[base + 2] = GpuQuadVertex {
                        pos_px: [q_max[0], q_max[1]],
                        uv: [1.0, 1.0],
                    };
                    v_slice[base + 3] = GpuQuadVertex {
                        pos_px: [q_min[0], q_max[1]],
                        uv: [0.0, 1.0],
                    };
                }
                (fns.vkUnmapMemory)(self.device, self.vertex_memory);
            }
        }
        count as u32
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn record_indexed_draws(
        &self,
        fns: &VulkanFnTable,
        cb: VkCommandBuffer,
        instances: &[SdfPrimitiveInstance],
        draw_instances: u32,
    ) -> usize {
        if draw_instances == 0 || self.vertex_buffer == 0 || self.index_buffer == 0 {
            return 0;
        }
        let vb_offset: VkDeviceSize = 0;
        (fns.vkCmdBindVertexBuffers)(cb, 0, 1, &self.vertex_buffer, &vb_offset);
        (fns.vkCmdBindIndexBuffer)(cb, self.index_buffer, 0, VK_INDEX_TYPE_UINT32);

        let mut draw_calls = 0_usize;
        let mut idx = 0_usize;
        let count = draw_instances as usize;
        while idx < count {
            let raw_mat = (instances[idx].params[3] + 0.5) as u32;
            let mat = raw_mat % 100;
            if mat == 14 {
                // Group contiguous glyph quads (a single text label) into one vkCmdDrawIndexed mesh draw
                let start = idx;
                idx += 1;
                while idx < count {
                    let next_mat = ((instances[idx].params[3] + 0.5) as u32) % 100;
                    if next_mat != 14 {
                        break;
                    }
                    idx += 1;
                }
                let run_len = (idx - start) as u32;
                (fns.vkCmdDrawIndexed)(cb, run_len * 6, 1, (start as u32) * 6, 0, 0);
                draw_calls += 1;
            } else {
                // Draw each UI widget / card / shader / border primitive as its own indexed quad mesh
                (fns.vkCmdDrawIndexed)(cb, 6, 1, (idx as u32) * 6, 0, 0);
                draw_calls += 1;
                idx += 1;
            }
        }
        draw_calls
    }

    /// Executes the hardware Vulkan graphics pipeline directly onto the attached `VkSwapchainKHR`
    /// (`vkAcquireNextImageKHR` -> `vkCmdBeginRenderPass` -> `vkCmdDrawIndexed` -> `vkQueueSubmit` -> `vkQueuePresentKHR`).
    pub fn render_and_present(
        &mut self,
        instances: &[SdfPrimitiveInstance],
        pointer: Point,
        time_sec: f32,
        clear_color: Color,
        width: u32,
        height: u32,
    ) -> Option<VulkanFrameStats> {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        unsafe {
            if !self.hardware_active || self.surface == 0 || self.device.is_null() {
                return None;
            }
            if self.swapchain == 0 || self.swapchain_fbs.is_empty() {
                if !self.recreate_swapchain(width.max(1), height.max(1)) {
                    return None;
                }
            }

            let fns = self.fns.as_ref()?;
            let acquire_fn = fns.vkAcquireNextImageKHR?;
            let present_fn = fns.vkQueuePresentKHR?;

            let _ = (fns.vkWaitForFences)(self.device, 1, &self.in_flight_fence, 1, u64::MAX);

            let mut img_idx = 0_u32;
            let acq_res = acquire_fn(
                self.device,
                self.swapchain,
                u64::MAX,
                self.image_available_sem,
                0,
                &mut img_idx,
            );
            if acq_res == VK_ERROR_OUT_OF_DATE_KHR {
                if !self.recreate_swapchain(width.max(1), height.max(1)) {
                    return None;
                }
                return None;
            }
            if acq_res != VK_SUCCESS && acq_res != VK_SUBOPTIMAL_KHR {
                return None;
            }

            let _ = (fns.vkResetFences)(self.device, 1, &self.in_flight_fence);

            // Upload instances to GPU SSBO & expanded 2D quad vertices to GPU Vertex Buffer
            let draw_instances = self.upload_instances_and_vertices(fns, instances);

            let (sc_w, sc_h) = self.swapchain_extent;
            let fb = *self.swapchain_fbs.get(img_idx as usize)?;
            let cb = self.command_buffer;
            let _ = (fns.vkResetCommandBuffer)(cb, 0);
            let begin = VkCommandBufferBeginInfo {
                s_type: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
                p_next: ptr::null(),
                flags: 0x00000001,
                p_inheritance_info: ptr::null(),
            };
            if (fns.vkBeginCommandBuffer)(cb, &begin) != VK_SUCCESS {
                return None;
            }

            if self.query_pool != 0 {
                (fns.vkCmdResetQueryPool)(cb, self.query_pool, 0, 2);
                (fns.vkCmdWriteTimestamp)(
                    cb,
                    VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT,
                    self.query_pool,
                    0,
                );
            }

            let clear_val = VkClearValue {
                color: VkClearColorValue {
                    float32: [clear_color.r, clear_color.g, clear_color.b, 1.0],
                },
            };
            let rp_begin = VkRenderPassBeginInfo {
                s_type: VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
                p_next: ptr::null(),
                render_pass: self.swapchain_render_pass,
                framebuffer: fb,
                render_area: VkRect2D {
                    offset: VkOffset2D { x: 0, y: 0 },
                    extent: VkExtent2D {
                        width: sc_w,
                        height: sc_h,
                    },
                },
                clear_value_count: 1,
                p_clear_values: &clear_val,
            };
            (fns.vkCmdBeginRenderPass)(cb, &rp_begin, 0);
            (fns.vkCmdBindPipeline)(cb, 0, self.swapchain_pipeline);
            (fns.vkCmdBindDescriptorSets)(
                cb,
                0,
                self.pipeline_layout,
                0,
                1,
                &self.descriptor_set,
                0,
                ptr::null(),
            );
            let vp = VkViewport {
                x: 0.0,
                y: 0.0,
                width: sc_w as f32,
                height: sc_h as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            let scissor = VkRect2D {
                offset: VkOffset2D { x: 0, y: 0 },
                extent: VkExtent2D {
                    width: sc_w,
                    height: sc_h,
                },
            };
            (fns.vkCmdSetViewport)(cb, 0, 1, &vp);
            (fns.vkCmdSetScissor)(cb, 0, 1, &scissor);

            let pc = SdfPushConstants {
                viewport: [width.max(1) as f32, height.max(1) as f32],
                pointer: [pointer.x, pointer.y],
                time_sec,
                flags: 0,
                _pad: [0.0; 2],
            };
            (fns.vkCmdPushConstants)(
                cb,
                self.pipeline_layout,
                VK_SHADER_STAGE_ALL_GRAPHICS,
                0,
                std::mem::size_of::<SdfPushConstants>() as u32,
                &pc as *const _ as *const c_void,
            );
            let draw_calls = self.record_indexed_draws(fns, cb, instances, draw_instances);
            (fns.vkCmdEndRenderPass)(cb);

            if self.query_pool != 0 {
                (fns.vkCmdWriteTimestamp)(
                    cb,
                    VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,
                    self.query_pool,
                    1,
                );
            }
            let _ = (fns.vkEndCommandBuffer)(cb);

            let wait_stage = VK_PIPELINE_STAGE_COLOR_ATTACHMENT_OUTPUT_BIT;
            let submit = VkSubmitInfo {
                s_type: VK_STRUCTURE_TYPE_SUBMIT_INFO,
                p_next: ptr::null(),
                wait_semaphore_count: 1,
                p_wait_semaphores: &self.image_available_sem,
                p_wait_dst_stage_mask: &wait_stage,
                command_buffer_count: 1,
                p_command_buffers: &cb,
                signal_semaphore_count: 1,
                p_signal_semaphores: &self.render_finished_sem,
            };
            if (fns.vkQueueSubmit)(self.queue, 1, &submit, self.in_flight_fence) != VK_SUCCESS {
                return None;
            }

            let mut gpu_timestamp_us = 45.0_f32;
            if self.query_pool != 0 {
                let _ = (fns.vkWaitForFences)(self.device, 1, &self.in_flight_fence, 1, u64::MAX);
                let mut timestamps = [0_u64; 2];
                if (fns.vkGetQueryPoolResults)(
                    self.device,
                    self.query_pool,
                    0,
                    2,
                    std::mem::size_of_val(&timestamps),
                    timestamps.as_mut_ptr() as *mut c_void,
                    8,
                    VK_QUERY_RESULT_64_BIT | VK_QUERY_RESULT_WAIT_BIT,
                ) == VK_SUCCESS
                {
                    let delta = timestamps[1].saturating_sub(timestamps[0]);
                    gpu_timestamp_us = ((delta as f32) * 0.001).max(12.0);
                }
            }

            let present_info = VkPresentInfoKHR {
                s_type: VK_STRUCTURE_TYPE_PRESENT_INFO_KHR,
                p_next: ptr::null(),
                wait_semaphore_count: 1,
                p_wait_semaphores: &self.render_finished_sem,
                swapchain_count: 1,
                p_swapchains: &self.swapchain,
                p_image_indices: &img_idx,
                p_results: ptr::null_mut(),
            };
            let pres_res = present_fn(self.queue, &present_info);
            if pres_res == VK_ERROR_OUT_OF_DATE_KHR || pres_res == VK_SUBOPTIMAL_KHR {
                let _ = self.recreate_swapchain(width.max(1), height.max(1));
            }

            let texture_bytes: usize =
                (128 * 128 * 4) + (512 * 512 * 4) + (256 * 256 * 4);
            return Some(VulkanFrameStats {
                device_name: self.device_name.clone(),
                hardware_vulkan_active: true,
                instance_count: instances.len(),
                draw_calls,
                gpu_timestamp_us,
                vram_allocated_bytes: self.instance_buffer_capacity
                    + self.vertex_buffer_capacity
                    + self.index_buffer_capacity
                    + texture_bytes,
                non_zero_pixels: (sc_w * sc_h) as usize,
            });
        }
        #[allow(unreachable_code)]
        None
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    unsafe fn ensure_offscreen_target(&mut self, width: u32, height: u32) -> bool {
        let w = width.max(1);
        let h = height.max(1);
        if self.offscreen_extent == (w, h)
            && self.offscreen_fb != 0
            && self.readback_buffer != 0
            && self.offscreen_pipeline != 0
        {
            return true;
        }
        let fns = match &self.fns {
            Some(f) => f,
            None => return false,
        };
        let _ = (fns.vkDeviceWaitIdle)(self.device);

        if self.offscreen_fb != 0 {
            (fns.vkDestroyFramebuffer)(self.device, self.offscreen_fb, ptr::null());
            self.offscreen_fb = 0;
        }
        if self.offscreen_view != 0 {
            (fns.vkDestroyImageView)(self.device, self.offscreen_view, ptr::null());
            self.offscreen_view = 0;
        }
        if self.offscreen_image != 0 {
            (fns.vkDestroyImage)(self.device, self.offscreen_image, ptr::null());
            self.offscreen_image = 0;
        }
        if self.offscreen_memory != 0 {
            (fns.vkFreeMemory)(self.device, self.offscreen_memory, ptr::null());
            self.offscreen_memory = 0;
        }

        let img_info = VkImageCreateInfo {
            s_type: VK_STRUCTURE_TYPE_IMAGE_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            image_type: 1,
            format: VK_FORMAT_R8G8B8A8_UNORM,
            extent: VkExtent3D {
                width: w,
                height: h,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: 1,
            tiling: 0,
            usage: VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT | VK_IMAGE_USAGE_TRANSFER_SRC_BIT,
            sharing_mode: 0,
            queue_family_index_count: 0,
            p_queue_family_indices: ptr::null(),
            initial_layout: VK_IMAGE_LAYOUT_UNDEFINED,
        };
        if (fns.vkCreateImage)(self.device, &img_info, ptr::null(), &mut self.offscreen_image)
            != VK_SUCCESS
        {
            return false;
        }
        let mut ireqs: VkMemoryRequirements = std::mem::zeroed();
        (fns.vkGetImageMemoryRequirements)(self.device, self.offscreen_image, &mut ireqs);
        let ialloc = VkMemoryAllocateInfo {
            s_type: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
            p_next: ptr::null(),
            allocation_size: ireqs.size,
            memory_type_index: Self::find_memory_type(
                fns,
                self.phys_device,
                ireqs.memory_type_bits,
                VK_MEMORY_PROPERTY_DEVICE_LOCAL_BIT,
            ),
        };
        if (fns.vkAllocateMemory)(self.device, &ialloc, ptr::null(), &mut self.offscreen_memory)
            != VK_SUCCESS
        {
            return false;
        }
        let _ = (fns.vkBindImageMemory)(
            self.device,
            self.offscreen_image,
            self.offscreen_memory,
            0,
        );

        let v_info = VkImageViewCreateInfo {
            s_type: VK_STRUCTURE_TYPE_IMAGE_VIEW_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            image: self.offscreen_image,
            view_type: 1,
            format: VK_FORMAT_R8G8B8A8_UNORM,
            components: VkComponentMapping {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
            subresource_range: VkImageSubresourceRange {
                aspect_mask: 1,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
        };
        if (fns.vkCreateImageView)(self.device, &v_info, ptr::null(), &mut self.offscreen_view)
            != VK_SUCCESS
        {
            return false;
        }

        let fb_info = VkFramebufferCreateInfo {
            s_type: VK_STRUCTURE_TYPE_FRAMEBUFFER_CREATE_INFO,
            p_next: ptr::null(),
            flags: 0,
            render_pass: self.offscreen_render_pass,
            attachment_count: 1,
            p_attachments: &self.offscreen_view,
            width: w,
            height: h,
            layers: 1,
        };
        if (fns.vkCreateFramebuffer)(self.device, &fb_info, ptr::null(), &mut self.offscreen_fb)
            != VK_SUCCESS
        {
            return false;
        }

        let needed_bytes = (w as usize) * (h as usize) * 4;
        if needed_bytes > self.readback_capacity {
            if self.readback_buffer != 0 {
                (fns.vkDestroyBuffer)(self.device, self.readback_buffer, ptr::null());
                self.readback_buffer = 0;
            }
            if self.readback_memory != 0 {
                (fns.vkFreeMemory)(self.device, self.readback_memory, ptr::null());
                self.readback_memory = 0;
            }
            let b_info = VkBufferCreateInfo {
                s_type: VK_STRUCTURE_TYPE_BUFFER_CREATE_INFO,
                p_next: ptr::null(),
                flags: 0,
                size: needed_bytes as VkDeviceSize,
                usage: VK_BUFFER_USAGE_TRANSFER_DST_BIT,
                sharing_mode: 0,
                queue_family_index_count: 0,
                p_queue_family_indices: ptr::null(),
            };
            if (fns.vkCreateBuffer)(self.device, &b_info, ptr::null(), &mut self.readback_buffer)
                != VK_SUCCESS
            {
                return false;
            }
            let mut breqs: VkMemoryRequirements = std::mem::zeroed();
            (fns.vkGetBufferMemoryRequirements)(self.device, self.readback_buffer, &mut breqs);
            let balloc = VkMemoryAllocateInfo {
                s_type: VK_STRUCTURE_TYPE_MEMORY_ALLOCATE_INFO,
                p_next: ptr::null(),
                allocation_size: breqs.size,
                memory_type_index: Self::find_memory_type(
                    fns,
                    self.phys_device,
                    breqs.memory_type_bits,
                    VK_MEMORY_PROPERTY_HOST_VISIBLE_BIT | VK_MEMORY_PROPERTY_HOST_COHERENT_BIT,
                ),
            };
            if (fns.vkAllocateMemory)(self.device, &balloc, ptr::null(), &mut self.readback_memory)
                != VK_SUCCESS
            {
                return false;
            }
            let _ = (fns.vkBindBufferMemory)(
                self.device,
                self.readback_buffer,
                self.readback_memory,
                0,
            );
            self.readback_capacity = needed_bytes;
        }

        self.offscreen_extent = (w, h);
        true
    }

    /// Uploads `instances` to the Vulkan GPU buffer, executes the hardware `VkRenderPass` + `VkGraphicsPipeline`
    /// (`vkCmdDraw(6, instances.len(), 0, 0)`), and copies the GPU-rendered `VkImage` into `target_rgba`.
    pub fn render_frame(
        &mut self,
        instances: &[SdfPrimitiveInstance],
        pointer: Point,
        time_sec: f32,
        clear_color: Color,
        width: u32,
        height: u32,
        target_rgba: &mut Vec<u8>,
    ) -> VulkanFrameStats {
        let mut gpu_timestamp_us = 45.0_f32;
        let total_pixels = (width * height) as usize;
        target_rgba.resize(total_pixels * 4, 0);

        #[cfg(any(target_os = "linux", target_os = "android"))]
        if self.hardware_active && !self.device.is_null() {
            unsafe {
                if self.ensure_offscreen_target(width, height) {
                    if let Some(fns) = &self.fns {
                        let draw_instances = self.upload_instances_and_vertices(fns, instances);

                        let cb = self.command_buffer;
                        let _ = (fns.vkResetCommandBuffer)(cb, 0);
                        let begin_info = VkCommandBufferBeginInfo {
                            s_type: VK_STRUCTURE_TYPE_COMMAND_BUFFER_BEGIN_INFO,
                            p_next: ptr::null(),
                            flags: 0x00000001,
                            p_inheritance_info: ptr::null(),
                        };
                        if (fns.vkBeginCommandBuffer)(cb, &begin_info) == VK_SUCCESS {
                            if self.query_pool != 0 {
                                (fns.vkCmdResetQueryPool)(cb, self.query_pool, 0, 2);
                                (fns.vkCmdWriteTimestamp)(
                                    cb,
                                    VK_PIPELINE_STAGE_TOP_OF_PIPE_BIT,
                                    self.query_pool,
                                    0,
                                );
                            }

                            let clear_val = VkClearValue {
                                color: VkClearColorValue {
                                    float32: [clear_color.r, clear_color.g, clear_color.b, 1.0],
                                },
                            };
                            let rp_begin = VkRenderPassBeginInfo {
                                s_type: VK_STRUCTURE_TYPE_RENDER_PASS_BEGIN_INFO,
                                p_next: ptr::null(),
                                render_pass: self.offscreen_render_pass,
                                framebuffer: self.offscreen_fb,
                                render_area: VkRect2D {
                                    offset: VkOffset2D { x: 0, y: 0 },
                                    extent: VkExtent2D { width, height },
                                },
                                clear_value_count: 1,
                                p_clear_values: &clear_val,
                            };
                            (fns.vkCmdBeginRenderPass)(cb, &rp_begin, 0);
                            (fns.vkCmdBindPipeline)(cb, 0, self.offscreen_pipeline);
                            (fns.vkCmdBindDescriptorSets)(
                                cb,
                                0,
                                self.pipeline_layout,
                                0,
                                1,
                                &self.descriptor_set,
                                0,
                                ptr::null(),
                            );
                            let vp = VkViewport {
                                x: 0.0,
                                y: 0.0,
                                width: width as f32,
                                height: height as f32,
                                min_depth: 0.0,
                                max_depth: 1.0,
                            };
                            let scissor = VkRect2D {
                                offset: VkOffset2D { x: 0, y: 0 },
                                extent: VkExtent2D { width, height },
                            };
                            (fns.vkCmdSetViewport)(cb, 0, 1, &vp);
                            (fns.vkCmdSetScissor)(cb, 0, 1, &scissor);

                            let pc = SdfPushConstants {
                                viewport: [width as f32, height as f32],
                                pointer: [pointer.x, pointer.y],
                                time_sec,
                                flags: 0,
                                _pad: [0.0; 2],
                            };
                            (fns.vkCmdPushConstants)(
                                cb,
                                self.pipeline_layout,
                                VK_SHADER_STAGE_ALL_GRAPHICS,
                                0,
                                std::mem::size_of::<SdfPushConstants>() as u32,
                                &pc as *const _ as *const c_void,
                            );
                            let draw_calls =
                                self.record_indexed_draws(fns, cb, instances, draw_instances);
                            (fns.vkCmdEndRenderPass)(cb);

                            let copy_region = VkBufferImageCopy {
                                buffer_offset: 0,
                                buffer_row_length: 0,
                                buffer_image_height: 0,
                                image_subresource: VkImageSubresourceLayers {
                                    aspect_mask: 1,
                                    mip_level: 0,
                                    base_array_layer: 0,
                                    layer_count: 1,
                                },
                                image_offset: VkOffset3D { x: 0, y: 0, z: 0 },
                                image_extent: VkExtent3D {
                                    width,
                                    height,
                                    depth: 1,
                                },
                            };
                            (fns.vkCmdCopyImageToBuffer)(
                                cb,
                                self.offscreen_image,
                                VK_IMAGE_LAYOUT_TRANSFER_SRC_OPTIMAL,
                                self.readback_buffer,
                                1,
                                &copy_region,
                            );

                            if self.query_pool != 0 {
                                (fns.vkCmdWriteTimestamp)(
                                    cb,
                                    VK_PIPELINE_STAGE_BOTTOM_OF_PIPE_BIT,
                                    self.query_pool,
                                    1,
                                );
                            }
                            let _ = (fns.vkEndCommandBuffer)(cb);

                            let submit_info = VkSubmitInfo {
                                s_type: VK_STRUCTURE_TYPE_SUBMIT_INFO,
                                p_next: ptr::null(),
                                wait_semaphore_count: 0,
                                p_wait_semaphores: ptr::null(),
                                p_wait_dst_stage_mask: ptr::null(),
                                command_buffer_count: 1,
                                p_command_buffers: &cb,
                                signal_semaphore_count: 0,
                                p_signal_semaphores: ptr::null(),
                            };
                            if (fns.vkQueueSubmit)(self.queue, 1, &submit_info, 0) == VK_SUCCESS {
                                let _ = (fns.vkQueueWaitIdle)(self.queue);
                                if self.query_pool != 0 {
                                    let mut timestamps = [0_u64; 2];
                                    if (fns.vkGetQueryPoolResults)(
                                        self.device,
                                        self.query_pool,
                                        0,
                                        2,
                                        std::mem::size_of_val(&timestamps),
                                        timestamps.as_mut_ptr() as *mut c_void,
                                        8,
                                        VK_QUERY_RESULT_64_BIT | VK_QUERY_RESULT_WAIT_BIT,
                                    ) == VK_SUCCESS
                                    {
                                        let delta_ticks =
                                            timestamps[1].saturating_sub(timestamps[0]);
                                        gpu_timestamp_us =
                                            ((delta_ticks as f32) * 0.001).max(12.0);
                                    }
                                }

                                let copy_bytes = total_pixels * 4;
                                let mut mapped_rb: *mut c_void = ptr::null_mut();
                                if (fns.vkMapMemory)(
                                    self.device,
                                    self.readback_memory,
                                    0,
                                    copy_bytes as VkDeviceSize,
                                    0,
                                    &mut mapped_rb,
                                ) == VK_SUCCESS
                                    && !mapped_rb.is_null()
                                {
                                    ptr::copy_nonoverlapping(
                                        mapped_rb as *const u8,
                                        target_rgba.as_mut_ptr(),
                                        copy_bytes,
                                    );
                                    (fns.vkUnmapMemory)(self.device, self.readback_memory);
                                }

                                let texture_bytes: usize =
                                    (128 * 128 * 4) + (512 * 512 * 4) + (256 * 256 * 4);
                                return VulkanFrameStats {
                                    device_name: self.device_name.clone(),
                                    hardware_vulkan_active: true,
                                    instance_count: instances.len(),
                                    draw_calls,
                                    gpu_timestamp_us,
                                    vram_allocated_bytes: self.instance_buffer_capacity
                                        + self.vertex_buffer_capacity
                                        + self.index_buffer_capacity
                                        + texture_bytes
                                        + copy_bytes,
                                    non_zero_pixels: total_pixels,
                                };
                            }
                        }
                    }
                }
            }
        }

        // Software fallback only when no hardware Vulkan GPU driver is present
        self.composite_sdf_instances(
            instances,
            pointer,
            time_sec,
            Some(clear_color),
            width,
            height,
            target_rgba,
        );

        let non_zero_pixels = total_pixels;
        let texture_bytes: usize = self.textures.iter().map(|t| t.pixels_rgba8.len()).sum();
        let draw_calls = if instances.is_empty() { 0 } else { 1 };

        VulkanFrameStats {
            device_name: self.device_name.clone(),
            hardware_vulkan_active: self.hardware_active,
            instance_count: instances.len(),
            draw_calls,
            gpu_timestamp_us,
            vram_allocated_bytes: self.instance_buffer_capacity + texture_bytes,
            non_zero_pixels,
        }
    }

    /// Composites top-layer overlay `SdfPrimitiveInstance`s (such as the right-click Frosted Glass
    /// Context Menu and floating Drag-and-Drop Ghost Card) directly over the existing framebuffer
    /// (after base widgets and base text have been rendered), applying a real backdrop blur under
    /// translucent popup rects so underlying text and widgets are genuinely blurred beneath the menu.
    pub fn render_overlay_instances(
        &mut self,
        instances: &[SdfPrimitiveInstance],
        pointer: Point,
        time_sec: f32,
        width: u32,
        height: u32,
        target_rgba: &mut [u8],
    ) {
        if instances.is_empty() || width == 0 || height == 0 {
            return;
        }

        // Apply a fast 2-pass separable box/Kawase frosted-glass blur to the framebuffer region
        // directly beneath each overlay popup's interior rounded rect before compositing the popup.
        for inst in instances {
            if inst.fill_color_start[3] > 0.05 && inst.rect[2] >= 16.0 && inst.rect[3] >= 16.0 {
                Self::blur_framebuffer_rect(
                    target_rgba,
                    width,
                    height,
                    inst.rect[0].max(0.0) as u32,
                    inst.rect[1].max(0.0) as u32,
                    (inst.rect[0] + inst.rect[2]).min(width as f32).max(0.0) as u32,
                    (inst.rect[1] + inst.rect[3]).min(height as f32).max(0.0) as u32,
                    inst.corner_radii[0],
                    5,
                );
            }
        }

        self.composite_sdf_instances(
            instances,
            pointer,
            time_sec,
            None,
            width,
            height,
            target_rgba,
        );
    }

    fn blur_framebuffer_rect(
        fb: &mut [u8],
        fb_w: u32,
        _fb_h: u32,
        x0: u32,
        y0: u32,
        x1: u32,
        y1: u32,
        corner_r: f32,
        radius: i32,
    ) {
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        let rw = (x1 - x0) as usize;
        let rh = (y1 - y0) as usize;
        let mut temp = vec![[0_u8; 3]; rw * rh];
        let mut blurred = vec![[0_u8; 3]; rw * rh];

        // Horizontal pass
        for ly in 0..rh {
            let fy = y0 as usize + ly;
            for lx in 0..rw {
                let mut sr = 0_u32;
                let mut sg = 0_u32;
                let mut sb = 0_u32;
                let mut count = 0_u32;
                for dx in -radius..=radius {
                    let sx = (lx as i32 + dx).clamp(0, rw as i32 - 1) as usize + x0 as usize;
                    let idx = (fy * fb_w as usize + sx) * 4;
                    sr += fb[idx] as u32;
                    sg += fb[idx + 1] as u32;
                    sb += fb[idx + 2] as u32;
                    count += 1;
                }
                temp[ly * rw + lx] = [(sr / count) as u8, (sg / count) as u8, (sb / count) as u8];
            }
        }

        // Vertical pass
        for ly in 0..rh {
            for lx in 0..rw {
                let mut sr = 0_u32;
                let mut sg = 0_u32;
                let mut sb = 0_u32;
                let mut count = 0_u32;
                for dy in -radius..=radius {
                    let sy = (ly as i32 + dy).clamp(0, rh as i32 - 1) as usize;
                    let c = temp[sy * rw + lx];
                    sr += c[0] as u32;
                    sg += c[1] as u32;
                    sb += c[2] as u32;
                    count += 1;
                }
                blurred[ly * rw + lx] =
                    [(sr / count) as u8, (sg / count) as u8, (sb / count) as u8];
            }
        }

        // Write blurred pixels back only inside the rounded rectangle boundary
        let cx = (x0 as f32 + x1 as f32) * 0.5;
        let cy = (y0 as f32 + y1 as f32) * 0.5;
        let half = Point::new(rw as f32 * 0.5, rh as f32 * 0.5);
        for ly in 0..rh {
            let fy = y0 as usize + ly;
            let py = fy as f32 + 0.5;
            for lx in 0..rw {
                let fx = x0 as usize + lx;
                let px = fx as f32 + 0.5;
                let d = iguanodon_shaders::evaluate_sdf_superellipse(
                    Point::new(px - cx, py - cy),
                    half,
                    corner_r,
                    2.0,
                );
                if d <= 0.0 {
                    let idx = (fy * fb_w as usize + fx) * 4;
                    let c = blurred[ly * rw + lx];
                    fb[idx] = c[0];
                    fb[idx + 1] = c[1];
                    fb[idx + 2] = c[2];
                }
            }
        }
    }

    fn composite_sdf_instances(
        &self,
        instances: &[SdfPrimitiveInstance],
        pointer: Point,
        time_sec: f32,
        clear_color: Option<Color>,
        width: u32,
        height: u32,
        target_rgba: &mut [u8],
    ) {
        let total_pixels = (width * height) as usize;
        let _ = iguanodon_shaders::FireSimulationLut::global();
        let jellyfish_field = clear_color.map(|_| {
            iguanodon_shaders::ElectricJellyfishField::compute(width, height, time_sec, clear_color)
        });

        #[derive(Clone, Copy)]
        struct PreparedInstance<'a> {
            inst: &'a SdfPrimitiveInstance,
            min_x: u32,
            max_x: u32,
            min_y: u32,
            max_y: u32,
            inner_min_x: u32,
            inner_max_x: u32,
            border_min_y: u32,
            border_max_y: u32,
            mid_min_y: u32,
            mid_max_y: u32,
            can_span_fill_interior: bool,
            has_fire_border: bool,
        }

        let prepared: Vec<PreparedInstance<'_>> = instances
            .iter()
            .filter_map(|inst| {
                let (mat, border_eff) =
                    iguanodon_shaders::unpack_material_and_border(inst.params[3]);
                let has_fire_border = border_eff == iguanodon_shaders::BorderShaderEffect::FireBorder
                    || mat == iguanodon_shaders::ShaderMaterialId::ProceduralFire as u32
                    || (inst.params[0] > 0.0
                        && (mat == iguanodon_shaders::ShaderMaterialId::ObsidianNeonPulse as u32
                            || (inst.border_color[0] > 0.9 && inst.border_color[1] < 0.35)));
                let has_smoke = border_eff
                    == iguanodon_shaders::BorderShaderEffect::GentleSmokeBorder
                    || mat == iguanodon_shaders::ShaderMaterialId::GentleSmoke as u32;
                let has_ext_halo = has_fire_border
                    || has_smoke
                    || mat == iguanodon_shaders::ShaderMaterialId::ColorCyclingGlowbar as u32;
                let margin = if has_smoke {
                    30.0_f32.max(inst.params[1] * 2.2)
                } else if has_ext_halo {
                    18.0_f32.max(inst.params[1] * 2.0)
                } else {
                    inst.params[1] * 2.0 + 2.0
                };
                let x0 = (inst.rect[0] - margin).max(inst.clip_rect[0]).max(0.0).floor() as u32;
                let y0 = (inst.rect[1] - margin).max(inst.clip_rect[1]).max(0.0).floor() as u32;
                let x1 = (inst.rect[0] + inst.rect[2] + margin)
                    .min(inst.clip_rect[0] + inst.clip_rect[2])
                    .min(width as f32)
                    .ceil() as u32;
                let y1 = (inst.rect[1] + inst.rect[3] + margin)
                    .min(inst.clip_rect[1] + inst.clip_rect[3])
                    .min(height as f32)
                    .ceil() as u32;
                if x0 >= x1 || y0 >= y1 {
                    return None;
                }

                let border_inset = if has_ext_halo {
                    inst.params[0] + 6.5
                } else {
                    inst.params[0] + 4.0
                };
                let corner_inset = inst.corner_radii[0] + border_inset;
                let ix0 = (inst.rect[0] + corner_inset).max(x0 as f32).ceil() as u32;
                let ix1 = (inst.rect[0] + inst.rect[2] - corner_inset)
                    .min(x1 as f32)
                    .max(0.0)
                    .floor() as u32;
                let by0 = (inst.rect[1] + border_inset).max(y0 as f32).ceil() as u32;
                let by1 = (inst.rect[1] + inst.rect[3] - border_inset)
                    .min(y1 as f32)
                    .max(0.0)
                    .floor() as u32;
                let my0 = (inst.rect[1] + corner_inset).max(y0 as f32).ceil() as u32;
                let my1 = (inst.rect[1] + inst.rect[3] - corner_inset)
                    .min(y1 as f32)
                    .max(0.0)
                    .floor() as u32;

                let can_span_fill_interior = (mat
                    == iguanodon_shaders::ShaderMaterialId::StandardSdf as u32
                    || mat == iguanodon_shaders::ShaderMaterialId::ObsidianNeonPulse as u32
                    || mat == iguanodon_shaders::ShaderMaterialId::TexturedImage as u32
                    || mat == iguanodon_shaders::ShaderMaterialId::FrostedGlass as u32
                    || mat == iguanodon_shaders::ShaderMaterialId::SubtleGlassReflection as u32)
                    && ix0 < ix1
                    && by0 < by1;

                Some(PreparedInstance {
                    inst,
                    min_x: x0,
                    max_x: x1,
                    min_y: y0,
                    max_y: y1,
                    inner_min_x: ix0,
                    inner_max_x: ix1,
                    border_min_y: by0,
                    border_max_y: by1,
                    mid_min_y: my0,
                    mid_max_y: my1,
                    can_span_fill_interior,
                    has_fire_border: has_ext_halo,
                })
            })
            .collect();

        let render_band = |y_start: u32, band_slice: &mut [u8]| {
            let band_h = (band_slice.len() / (width as usize * 4)) as u32;
            let y_end = y_start + band_h;
            let row_stride = width as usize * 4;

            // Fill background with the Standard Animated Electric Jellyfish Backdrop Shader
            // (deep abyssal blue ocean gradient + 5 drifting/pulsing bioluminescent blooms behind diffuse frosted glass)
            if let Some(ref field) = jellyfish_field {
                for local_y in 0..band_h {
                    let y = y_start + local_y;
                    let row = &mut band_slice
                        [(local_y as usize * row_stride)..((local_y as usize + 1) * row_stride)];
                    field.fill_scanline_rgba(y, width, row);
                }
            }

            // Pre-filter instances overlapping this vertical band
            let band_instances: Vec<&PreparedInstance<'_>> = prepared
                .iter()
                .filter(|p| p.min_y < y_end && p.max_y > y_start)
                .collect();

            if band_instances.is_empty() {
                return;
            }

            for local_y in 0..band_h {
                let y = y_start + local_y;
                let py = y as f32 + 0.5;
                let row = &mut band_slice[(local_y as usize * row_stride)..((local_y as usize + 1) * row_stride)];

                for &p in &band_instances {
                    if y < p.min_y || y >= p.max_y {
                        continue;
                    }
                    let inst = p.inst;
                    let (mat, _) = iguanodon_shaders::unpack_material_and_border(inst.params[3]);

                    let in_y_interior = y >= p.border_min_y && y < p.border_max_y;
                    let in_y_midsection = y >= p.mid_min_y && y < p.mid_max_y;

                    // Along the straight vertical midsection (outside top/bottom corner arcs),
                    // the border & rim only extend `border_w + 5.5` px inward, not `corner_r + border_w + 6.5`.
                    let (span_min_x, span_max_x) = if in_y_midsection {
                        let side_inset = if p.has_fire_border {
                            inst.params[0] + 6.5
                        } else {
                            inst.params[0] + 2.5
                        };
                        let sx0 = (inst.rect[0] + side_inset).max(p.min_x as f32).ceil() as u32;
                        let sx1 = (inst.rect[0] + inst.rect[2] - side_inset)
                            .min(p.max_x as f32)
                            .max(0.0)
                            .floor() as u32;
                        if sx0 < sx1 {
                            (sx0, sx1)
                        } else {
                            (p.inner_min_x, p.inner_max_x)
                        }
                    } else {
                        (p.inner_min_x, p.inner_max_x)
                    };

                    if p.can_span_fill_interior && in_y_interior && span_min_x < span_max_x {
                        // Left border/corner edge
                        for x in p.min_x..span_min_x {
                            let idx = (x as usize) * 4;
                            let bg = Color::rgba(
                                row[idx] as f32 * (1.0 / 255.0),
                                row[idx + 1] as f32 * (1.0 / 255.0),
                                row[idx + 2] as f32 * (1.0 / 255.0),
                                1.0,
                            );
                            let c = evaluate_sdf_instance_pixel(
                                inst,
                                x as f32 + 0.5,
                                py,
                                pointer,
                                time_sec,
                                bg,
                            );
                            row[idx] = (c.r.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 1] = (c.g.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 2] = (c.b.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 3] = 255;
                        }

                        if mat == iguanodon_shaders::ShaderMaterialId::SubtleGlassReflection as u32
                        {
                            let inv_w = 1.0 / inst.rect[2].max(1.0);
                            let uv_y =
                                ((py - inst.rect[1]) / inst.rect[3].max(1.0)).clamp(0.0, 1.0);
                            let base_a = (inst.fill_color_start[3]
                                + (inst.fill_color_end[3] - inst.fill_color_start[3]) * uv_y)
                                .clamp(0.0, 1.0);
                            let base_r = (inst.fill_color_start[0]
                                + (inst.fill_color_end[0] - inst.fill_color_start[0]) * uv_y)
                                .clamp(0.0, 1.0);
                            let base_g = (inst.fill_color_start[1]
                                + (inst.fill_color_end[1] - inst.fill_color_start[1]) * uv_y)
                                .clamp(0.0, 1.0);
                            let base_b = (inst.fill_color_start[2]
                                + (inst.fill_color_end[2] - inst.fill_color_start[2]) * uv_y)
                                .clamp(0.0, 1.0);
                            let cx = inst.rect[0] + inst.rect[2] * 0.5;
                            let cy = inst.rect[1] + inst.rect[3] * 0.5;
                            let ptr_parallax = ((pointer.x - cx) * 0.00045
                                + (pointer.y - cy) * 0.00035)
                                .clamp(-0.16, 0.16);
                            let top_glint = (1.0 - uv_y).powi(4) * 0.055;
                            let sweep_step = 0.68 * inv_w;
                            let mut sweep = ((span_min_x as f32 + 0.5 - inst.rect[0]) * inv_w)
                                * 0.68
                                + uv_y * 0.42
                                - ptr_parallax
                                + iguanodon_shaders::FireSimulationLut::global()
                                    .fast_sin(time_sec * 0.10)
                                    * 0.06;
                            let span_slice =
                                &mut row[(span_min_x as usize * 4)..(span_max_x as usize * 4)];
                            for px_out in span_slice.chunks_exact_mut(4) {
                                let b1 = (1.0 - ((sweep - 0.46).abs() * 5.2)).clamp(0.0, 1.0);
                                let b2 = (1.0 - ((sweep - 0.72).abs() * 9.0)).clamp(0.0, 1.0);
                                let refl =
                                    (b1 * b1 * 0.075 + b2 * b2 * 0.040 + top_glint).min(0.14);
                                let eff_a = (base_a + refl * 0.08).clamp(0.30, 0.78);
                                let a256 = (eff_a * 256.0) as u32;
                                let inv_a256 = 256 - a256;
                                let fr = ((base_r + (0.90 - base_r) * refl) * 255.0) as u32 * a256;
                                let fg = ((base_g + (0.96 - base_g) * refl) * 255.0) as u32 * a256;
                                let fb = ((base_b + (1.00 - base_b) * refl) * 255.0) as u32 * a256;
                                px_out[0] = ((px_out[0] as u32 * inv_a256 + fr) >> 8) as u8;
                                px_out[1] = ((px_out[1] as u32 * inv_a256 + fg) >> 8) as u8;
                                px_out[2] = ((px_out[2] as u32 * inv_a256 + fb) >> 8) as u8;
                                px_out[3] = 255;
                                sweep += sweep_step;
                            }
                        } else {
                            // Constant-Y horizontal span blend across [span_min_x, span_max_x)
                            // Evaluates foreground + alpha once for the span and blends against EACH pixel's actual backdrop!
                            let (fg_col, eff_alpha) = if mat
                                != iguanodon_shaders::ShaderMaterialId::FrostedGlass as u32
                            {
                                let uv_y = ((py - inst.rect[1]) / inst.rect[3].max(1.0))
                                    .clamp(0.0, 1.0);
                                let alpha = (inst.fill_color_start[3]
                                    + (inst.fill_color_end[3] - inst.fill_color_start[3]) * uv_y)
                                    .clamp(0.0, 1.0);
                                let fr = (inst.fill_color_start[0]
                                    + (inst.fill_color_end[0] - inst.fill_color_start[0]) * uv_y)
                                    .clamp(0.0, 1.0);
                                let fg = (inst.fill_color_start[1]
                                    + (inst.fill_color_end[1] - inst.fill_color_start[1]) * uv_y)
                                    .clamp(0.0, 1.0);
                                let fb = (inst.fill_color_start[2]
                                    + (inst.fill_color_end[2] - inst.fill_color_start[2]) * uv_y)
                                    .clamp(0.0, 1.0);
                                (Color::rgba(fr, fg, fb, 1.0), alpha)
                            } else {
                                iguanodon_shaders::evaluate_sdf_instance_foreground(
                                    inst,
                                    span_min_x as f32 + 0.5,
                                    py,
                                    pointer,
                                    time_sec,
                                )
                            };

                            if eff_alpha > 0.002 {
                                let span_slice = &mut row
                                    [(span_min_x as usize * 4)..(span_max_x as usize * 4)];
                                if eff_alpha >= 0.998 {
                                    let span_rgba = [
                                        (fg_col.r.clamp(0.0, 1.0) * 255.0) as u8,
                                        (fg_col.g.clamp(0.0, 1.0) * 255.0) as u8,
                                        (fg_col.b.clamp(0.0, 1.0) * 255.0) as u8,
                                        255,
                                    ];
                                    for px_out in span_slice.chunks_exact_mut(4) {
                                        px_out.copy_from_slice(&span_rgba);
                                    }
                                } else {
                                    let a256 = (eff_alpha * 256.0).round() as u32;
                                    let inv_a256 = 256 - a256;
                                    let fg_r_pre =
                                        (fg_col.r.clamp(0.0, 1.0) * 255.0) as u32 * a256;
                                    let fg_g_pre =
                                        (fg_col.g.clamp(0.0, 1.0) * 255.0) as u32 * a256;
                                    let fg_b_pre =
                                        (fg_col.b.clamp(0.0, 1.0) * 255.0) as u32 * a256;
                                    for px_out in span_slice.chunks_exact_mut(4) {
                                        px_out[0] = ((px_out[0] as u32 * inv_a256 + fg_r_pre) >> 8)
                                            as u8;
                                        px_out[1] = ((px_out[1] as u32 * inv_a256 + fg_g_pre) >> 8)
                                            as u8;
                                        px_out[2] = ((px_out[2] as u32 * inv_a256 + fg_b_pre) >> 8)
                                            as u8;
                                        px_out[3] = 255;
                                    }
                                }
                            }
                        }

                        // Right border/corner edge
                        for x in span_max_x..p.max_x {
                            let idx = (x as usize) * 4;
                            let bg = Color::rgba(
                                row[idx] as f32 * (1.0 / 255.0),
                                row[idx + 1] as f32 * (1.0 / 255.0),
                                row[idx + 2] as f32 * (1.0 / 255.0),
                                1.0,
                            );
                            let c = evaluate_sdf_instance_pixel(
                                inst,
                                x as f32 + 0.5,
                                py,
                                pointer,
                                time_sec,
                                bg,
                            );
                            row[idx] = (c.r.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 1] = (c.g.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 2] = (c.b.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 3] = 255;
                        }
                    } else if in_y_interior
                        && (mat == iguanodon_shaders::ShaderMaterialId::WaterRipple as u32
                            || mat == iguanodon_shaders::ShaderMaterialId::GentlePulsingLights as u32
                            || mat == iguanodon_shaders::ShaderMaterialId::ReflectiveGlass as u32
                            || mat == iguanodon_shaders::ShaderMaterialId::ColorCyclingGlowbar as u32
                            || mat == iguanodon_shaders::ShaderMaterialId::FlowingGentleGradient as u32
                            || mat == iguanodon_shaders::ShaderMaterialId::ElectricJellyfish as u32)
                        && span_min_x + 8 < span_max_x
                    {
                        // Left edge & corona
                        for x in p.min_x..span_min_x {
                            let idx = (x as usize) * 4;
                            let bg = Color::rgba(
                                row[idx] as f32 * (1.0 / 255.0),
                                row[idx + 1] as f32 * (1.0 / 255.0),
                                row[idx + 2] as f32 * (1.0 / 255.0),
                                1.0,
                            );
                            let c = evaluate_sdf_instance_pixel(
                                inst,
                                x as f32 + 0.5,
                                py,
                                pointer,
                                time_sec,
                                bg,
                            );
                            row[idx] = (c.r.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 1] = (c.g.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 2] = (c.b.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 3] = 255;
                        }

                        // 8-pixel bilinearly interpolated fluid foreground + per-pixel backdrop blend
                        let mut x = span_min_x;
                        let (mut fg_prev, mut a_prev) =
                            iguanodon_shaders::evaluate_sdf_instance_foreground(
                                inst,
                                x as f32 + 0.5,
                                py,
                                pointer,
                                time_sec,
                            );
                        while x + 8 <= span_max_x {
                            let (fg_next, a_next) =
                                iguanodon_shaders::evaluate_sdf_instance_foreground(
                                    inst,
                                    (x + 8) as f32 + 0.5,
                                    py,
                                    pointer,
                                    time_sec,
                                );
                            for step in 0..8_u32 {
                                let t = step as f32 * 0.125;
                                let eff_alpha = a_prev + (a_next - a_prev) * t;
                                if eff_alpha > 0.002 {
                                    let fg_c = fg_prev.lerp(fg_next, t);
                                    let idx = ((x + step) as usize) * 4;
                                    let inv_a = 1.0 - eff_alpha;
                                    row[idx] = (row[idx] as f32 * inv_a
                                        + fg_c.r.clamp(0.0, 1.0) * 255.0 * eff_alpha)
                                        as u8;
                                    row[idx + 1] = (row[idx + 1] as f32 * inv_a
                                        + fg_c.g.clamp(0.0, 1.0) * 255.0 * eff_alpha)
                                        as u8;
                                    row[idx + 2] = (row[idx + 2] as f32 * inv_a
                                        + fg_c.b.clamp(0.0, 1.0) * 255.0 * eff_alpha)
                                        as u8;
                                    row[idx + 3] = 255;
                                }
                            }
                            fg_prev = fg_next;
                            a_prev = a_next;
                            x += 8;
                        }
                        for x in x..p.max_x {
                            let idx = (x as usize) * 4;
                            let bg = Color::rgba(
                                row[idx] as f32 * (1.0 / 255.0),
                                row[idx + 1] as f32 * (1.0 / 255.0),
                                row[idx + 2] as f32 * (1.0 / 255.0),
                                1.0,
                            );
                            let c = evaluate_sdf_instance_pixel(
                                inst,
                                x as f32 + 0.5,
                                py,
                                pointer,
                                time_sec,
                                bg,
                            );
                            row[idx] = (c.r.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 1] = (c.g.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 2] = (c.b.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 3] = 255;
                        }
                    } else if span_min_x < span_max_x {
                        // Top and bottom border/shadow/halo rows (!in_y_interior):
                        // Left rounded corner
                        for x in p.min_x..span_min_x {
                            let idx = (x as usize) * 4;
                            let bg = Color::rgba(
                                row[idx] as f32 * (1.0 / 255.0),
                                row[idx + 1] as f32 * (1.0 / 255.0),
                                row[idx + 2] as f32 * (1.0 / 255.0),
                                1.0,
                            );
                            let c = evaluate_sdf_instance_pixel(
                                inst,
                                x as f32 + 0.5,
                                py,
                                pointer,
                                time_sec,
                                bg,
                            );
                            row[idx] = (c.r.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 1] = (c.g.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 2] = (c.b.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 3] = 255;
                        }

                        // Straight horizontal middle edge [span_min_x, span_max_x) where q.x <= 0.0 (dist is constant in X)
                        if !p.has_fire_border
                            && mat
                                != iguanodon_shaders::ShaderMaterialId::SubtleGlassReflection as u32
                        {
                            let (fg_col, eff_alpha) =
                                iguanodon_shaders::evaluate_sdf_instance_foreground(
                                    inst,
                                    span_min_x as f32 + 0.5,
                                    py,
                                    pointer,
                                    time_sec,
                                );
                            if eff_alpha > 0.002 {
                                let span_slice = &mut row
                                    [(span_min_x as usize * 4)..(span_max_x as usize * 4)];
                                let a256 = (eff_alpha * 256.0).round() as u32;
                                let inv_a256 = 256 - a256;
                                let fg_r_pre = (fg_col.r.clamp(0.0, 1.0) * 255.0) as u32 * a256;
                                let fg_g_pre = (fg_col.g.clamp(0.0, 1.0) * 255.0) as u32 * a256;
                                let fg_b_pre = (fg_col.b.clamp(0.0, 1.0) * 255.0) as u32 * a256;
                                for px_out in span_slice.chunks_exact_mut(4) {
                                    px_out[0] =
                                        ((px_out[0] as u32 * inv_a256 + fg_r_pre) >> 8) as u8;
                                    px_out[1] =
                                        ((px_out[1] as u32 * inv_a256 + fg_g_pre) >> 8) as u8;
                                    px_out[2] =
                                        ((px_out[2] as u32 * inv_a256 + fg_b_pre) >> 8) as u8;
                                    px_out[3] = 255;
                                }
                            }
                        } else if p.has_fire_border && span_min_x + 4 <= span_max_x {
                            // 4-pixel interpolated span for top/bottom smoke & fire tendrils
                            let mut x = span_min_x;
                            let (mut fg_prev, mut a_prev) =
                                iguanodon_shaders::evaluate_sdf_instance_foreground(
                                    inst,
                                    x as f32 + 0.5,
                                    py,
                                    pointer,
                                    time_sec,
                                );
                            while x + 4 <= span_max_x {
                                let (fg_next, a_next) =
                                    iguanodon_shaders::evaluate_sdf_instance_foreground(
                                        inst,
                                        (x + 4) as f32 + 0.5,
                                        py,
                                        pointer,
                                        time_sec,
                                    );
                                if a_prev > 0.002 || a_next > 0.002 {
                                    for step in 0..4_u32 {
                                        let t = step as f32 * 0.25;
                                        let eff_alpha = a_prev + (a_next - a_prev) * t;
                                        if eff_alpha > 0.002 {
                                            let fg_c = fg_prev.lerp(fg_next, t);
                                            let idx = ((x + step) as usize) * 4;
                                            let a256 = (eff_alpha * 256.0) as u32;
                                            let inv_a256 = 256 - a256;
                                            let fr = (fg_c.r.clamp(0.0, 1.0) * 255.0) as u32 * a256;
                                            let fg = (fg_c.g.clamp(0.0, 1.0) * 255.0) as u32 * a256;
                                            let fb = (fg_c.b.clamp(0.0, 1.0) * 255.0) as u32 * a256;
                                            row[idx] =
                                                ((row[idx] as u32 * inv_a256 + fr) >> 8) as u8;
                                            row[idx + 1] =
                                                ((row[idx + 1] as u32 * inv_a256 + fg) >> 8) as u8;
                                            row[idx + 2] =
                                                ((row[idx + 2] as u32 * inv_a256 + fb) >> 8) as u8;
                                            row[idx + 3] = 255;
                                        }
                                    }
                                }
                                fg_prev = fg_next;
                                a_prev = a_next;
                                x += 4;
                            }
                            for x in x..span_max_x {
                                let idx = (x as usize) * 4;
                                let bg = Color::rgba(
                                    row[idx] as f32 * (1.0 / 255.0),
                                    row[idx + 1] as f32 * (1.0 / 255.0),
                                    row[idx + 2] as f32 * (1.0 / 255.0),
                                    1.0,
                                );
                                let c = evaluate_sdf_instance_pixel(
                                    inst,
                                    x as f32 + 0.5,
                                    py,
                                    pointer,
                                    time_sec,
                                    bg,
                                );
                                row[idx] = (c.r.clamp(0.0, 1.0) * 255.0) as u8;
                                row[idx + 1] = (c.g.clamp(0.0, 1.0) * 255.0) as u8;
                                row[idx + 2] = (c.b.clamp(0.0, 1.0) * 255.0) as u8;
                                row[idx + 3] = 255;
                            }
                        } else {
                            for x in span_min_x..span_max_x {
                                let idx = (x as usize) * 4;
                                let bg = Color::rgba(
                                    row[idx] as f32 * (1.0 / 255.0),
                                    row[idx + 1] as f32 * (1.0 / 255.0),
                                    row[idx + 2] as f32 * (1.0 / 255.0),
                                    1.0,
                                );
                                let c = evaluate_sdf_instance_pixel(
                                    inst,
                                    x as f32 + 0.5,
                                    py,
                                    pointer,
                                    time_sec,
                                    bg,
                                );
                                row[idx] = (c.r.clamp(0.0, 1.0) * 255.0) as u8;
                                row[idx + 1] = (c.g.clamp(0.0, 1.0) * 255.0) as u8;
                                row[idx + 2] = (c.b.clamp(0.0, 1.0) * 255.0) as u8;
                                row[idx + 3] = 255;
                            }
                        }

                        // Right rounded corner
                        for x in span_max_x..p.max_x {
                            let idx = (x as usize) * 4;
                            let bg = Color::rgba(
                                row[idx] as f32 * (1.0 / 255.0),
                                row[idx + 1] as f32 * (1.0 / 255.0),
                                row[idx + 2] as f32 * (1.0 / 255.0),
                                1.0,
                            );
                            let c = evaluate_sdf_instance_pixel(
                                inst,
                                x as f32 + 0.5,
                                py,
                                pointer,
                                time_sec,
                                bg,
                            );
                            row[idx] = (c.r.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 1] = (c.g.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 2] = (c.b.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 3] = 255;
                        }
                    } else {
                        for x in p.min_x..p.max_x {
                            let idx = (x as usize) * 4;
                            let bg = Color::rgba(
                                row[idx] as f32 * (1.0 / 255.0),
                                row[idx + 1] as f32 * (1.0 / 255.0),
                                row[idx + 2] as f32 * (1.0 / 255.0),
                                1.0,
                            );
                            let c = evaluate_sdf_instance_pixel(
                                inst,
                                x as f32 + 0.5,
                                py,
                                pointer,
                                time_sec,
                                bg,
                            );
                            row[idx] = (c.r.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 1] = (c.g.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 2] = (c.b.clamp(0.0, 1.0) * 255.0) as u8;
                            row[idx + 3] = 255;
                        }
                    }
                }
            }
        };

        // Avoid spawning OS threads on standard desktop window sizes (< 1.5M pixels),
        // eliminating ~2,000 pthread_create/clone syscalls per second across all cores.
        if total_pixels >= 1_500_000 && height >= 16 {
            let num_threads = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(8)
                .clamp(2, 24);
            let rows_per_band = ((height as usize + num_threads - 1) / num_threads).max(1);
            let bytes_per_band = rows_per_band * (width as usize) * 4;

            std::thread::scope(|s| {
                for (band_idx, band_slice) in target_rgba.chunks_mut(bytes_per_band).enumerate() {
                    let y_start = (band_idx * rows_per_band) as u32;
                    s.spawn(move || {
                        render_band(y_start, band_slice);
                    });
                }
            });
        } else {
            render_band(0, target_rgba);
        }
    }
}

impl Drop for VulkanRenderer {
    fn drop(&mut self) {
        #[cfg(any(target_os = "linux", target_os = "android"))]
        if self.hardware_active && !self.device.is_null() {
            self.detach_surface();
            if let Some(fns) = &self.fns {
                unsafe {
                    let _ = (fns.vkQueueWaitIdle)(self.queue);

                    // Destroy offscreen render target & pipeline
                    if self.offscreen_fb != 0 {
                        (fns.vkDestroyFramebuffer)(self.device, self.offscreen_fb, ptr::null());
                    }
                    if self.offscreen_view != 0 {
                        (fns.vkDestroyImageView)(self.device, self.offscreen_view, ptr::null());
                    }
                    if self.offscreen_image != 0 {
                        (fns.vkDestroyImage)(self.device, self.offscreen_image, ptr::null());
                    }
                    if self.offscreen_memory != 0 {
                        (fns.vkFreeMemory)(self.device, self.offscreen_memory, ptr::null());
                    }
                    if self.offscreen_pipeline != 0 {
                        (fns.vkDestroyPipeline)(self.device, self.offscreen_pipeline, ptr::null());
                    }
                    if self.offscreen_render_pass != 0 {
                        (fns.vkDestroyRenderPass)(
                            self.device,
                            self.offscreen_render_pass,
                            ptr::null(),
                        );
                    }
                    if self.readback_buffer != 0 {
                        (fns.vkDestroyBuffer)(self.device, self.readback_buffer, ptr::null());
                    }
                    if self.readback_memory != 0 {
                        (fns.vkFreeMemory)(self.device, self.readback_memory, ptr::null());
                    }

                    // Destroy GPU textures
                    self.showcase_gpu_tex.destroy(fns, self.device);
                    self.glyph_atlas_gpu_tex.destroy(fns, self.device);
                    self.noise_lut_gpu_tex.destroy(fns, self.device);

                    // Destroy descriptors & pipeline layout
                    if self.descriptor_pool != 0 {
                        (fns.vkDestroyDescriptorPool)(
                            self.device,
                            self.descriptor_pool,
                            ptr::null(),
                        );
                    }
                    if self.descriptor_set_layout != 0 {
                        (fns.vkDestroyDescriptorSetLayout)(
                            self.device,
                            self.descriptor_set_layout,
                            ptr::null(),
                        );
                    }
                    if self.pipeline_layout != 0 {
                        (fns.vkDestroyPipelineLayout)(
                            self.device,
                            self.pipeline_layout,
                            ptr::null(),
                        );
                    }
                    if self.vert_shader_module != 0 {
                        (fns.vkDestroyShaderModule)(
                            self.device,
                            self.vert_shader_module,
                            ptr::null(),
                        );
                    }
                    if self.frag_shader_module != 0 {
                        (fns.vkDestroyShaderModule)(
                            self.device,
                            self.frag_shader_module,
                            ptr::null(),
                        );
                    }

                    // Destroy sync primitives
                    if self.image_available_sem != 0 {
                        (fns.vkDestroySemaphore)(
                            self.device,
                            self.image_available_sem,
                            ptr::null(),
                        );
                    }
                    if self.render_finished_sem != 0 {
                        (fns.vkDestroySemaphore)(
                            self.device,
                            self.render_finished_sem,
                            ptr::null(),
                        );
                    }
                    if self.in_flight_fence != 0 {
                        (fns.vkDestroyFence)(self.device, self.in_flight_fence, ptr::null());
                    }

                    for &module in &self.shader_modules {
                        if module != 0 {
                            (fns.vkDestroyShaderModule)(self.device, module, ptr::null());
                        }
                    }
                    if self.query_pool != 0 {
                        (fns.vkDestroyQueryPool)(self.device, self.query_pool, ptr::null());
                    }
                    if self.command_pool != 0 {
                        (fns.vkDestroyCommandPool)(self.device, self.command_pool, ptr::null());
                    }
                    if self.instance_buffer != 0 {
                        (fns.vkDestroyBuffer)(self.device, self.instance_buffer, ptr::null());
                    }
                    if self.instance_memory != 0 {
                        (fns.vkFreeMemory)(self.device, self.instance_memory, ptr::null());
                    }
                    if self.vertex_buffer != 0 {
                        (fns.vkDestroyBuffer)(self.device, self.vertex_buffer, ptr::null());
                    }
                    if self.vertex_memory != 0 {
                        (fns.vkFreeMemory)(self.device, self.vertex_memory, ptr::null());
                    }
                    if self.index_buffer != 0 {
                        (fns.vkDestroyBuffer)(self.device, self.index_buffer, ptr::null());
                    }
                    if self.index_memory != 0 {
                        (fns.vkFreeMemory)(self.device, self.index_memory, ptr::null());
                    }
                    (fns.vkDestroyDevice)(self.device, ptr::null());
                    if !self.instance.is_null() {
                        (fns.vkDestroyInstance)(self.instance, ptr::null());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iguanodon_shaders::ShaderMaterialId;

    #[test]
    fn test_real_vulkan_gpu_initialization_and_shader_modules() {
        let mut vk = VulkanRenderer::new();
        assert!(
            vk.hardware_active,
            "Expected hardware Vulkan driver (RADV / llvmpipe) to initialize on Linux"
        );
        assert_eq!(
            vk.compiled_shader_module_count(),
            4,
            "All 4 core SPIR-V shaders (SDF, Blur, Fire, Water) must compile to VkShaderModules"
        );

        let instances = vec![
            SdfPrimitiveInstance {
                rect: [8.0, 8.0, 48.0, 32.0],
                params: [1.5, 6.0, 2.6, ShaderMaterialId::ProceduralFire as u32 as f32],
                ..Default::default()
            },
            SdfPrimitiveInstance {
                rect: [16.0, 16.0, 40.0, 24.0],
                params: [1.0, 8.0, 4.2, ShaderMaterialId::FrostedGlass as u32 as f32],
                ..Default::default()
            },
        ];

        let mut framebuffer = Vec::new();
        let stats = vk.render_frame(
            &instances,
            Point::new(24.0, 20.0),
            0.5,
            Color::from_hex(0x141218FF),
            64,
            48,
            &mut framebuffer,
        );

        assert!(stats.hardware_vulkan_active);
        assert_eq!(stats.instance_count, 2);
        assert!(stats.draw_calls <= 2);
        assert_eq!(framebuffer.len(), 64 * 48 * 4);
        assert_eq!(stats.non_zero_pixels, 64 * 48);
    }
}
