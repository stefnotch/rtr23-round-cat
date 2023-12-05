use std::ffi::CStr;

use ash::{
    extensions::khr::{
        AccelerationStructure, BufferDeviceAddress, RayTracingPipeline, Synchronization2,
    },
    vk::{self, ApplicationInfo, DeviceCreateInfo, DeviceQueueCreateInfo, InstanceCreateInfo},
};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::{event_loop::EventLoop, window::Window};

pub struct Context {
    _entry: ash::Entry,
    pub instance: ash::Instance,

    pub surface_loader: ash::extensions::khr::Surface,
    pub surface: vk::SurfaceKHR,

    pub context_raytracing: ContextRaytracing,
    pub synchronisation2_loader: ash::extensions::khr::Synchronization2,

    pub physical_device: vk::PhysicalDevice,
    pub queue_family_index: u32,

    pub device: ash::Device,
    pub queue: vk::Queue,

    pub buffer_device_address: BufferDeviceAddress,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
}

pub struct ContextRaytracing {
    pub ray_tracing_pipeline: RayTracingPipeline,
    pub physical_device_ray_tracing_pipeline_properties_khr:
        vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,

    pub acceleration_structure: AccelerationStructure,
    pub physical_device_acceleration_structure_properties_khr:
        vk::PhysicalDeviceAccelerationStructurePropertiesKHR,
}

impl Context {
    pub fn new(event_loop: &EventLoop<()>, window: &Window) -> Self {
        let entry = unsafe { ash::Entry::load() }.expect("Could not load vulkan library");

        let instance = {
            let surface_extension =
                ash_window::enumerate_required_extensions(event_loop.raw_display_handle()).unwrap();

            let app_info = ApplicationInfo::builder().api_version(vk::API_VERSION_1_3);
            let create_info = InstanceCreateInfo::builder()
                .application_info(&app_info)
                .enabled_extension_names(surface_extension);
            unsafe { entry.create_instance(&create_info, None) }.expect("Could not create instance")
        };

        let (surface, surface_loader) = {
            let surface = unsafe {
                ash_window::create_surface(
                    &entry,
                    &instance,
                    window.raw_display_handle(),
                    window.raw_window_handle(),
                    None,
                )
            }
            .expect("Could not create surface");

            let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);

            (surface, surface_loader)
        };

        let (physical_device, queue_family_index) =
            find_physical_device(&instance, &surface, &surface_loader);

        let device = create_logical_device(&instance, &physical_device);

        let queue = unsafe { device.get_device_queue(queue_family_index, 0) };

        let synchronisation2_loader = Synchronization2::new(&instance, &device);

        let ray_tracing_pipeline = RayTracingPipeline::new(&instance, &device);
        let physical_device_ray_tracing_pipeline_properties_khr =
            unsafe { RayTracingPipeline::get_properties(&instance, physical_device) };

        let acceleration_structure = AccelerationStructure::new(&instance, &device);
        let physical_device_acceleration_structure_properties_khr =
            unsafe { AccelerationStructure::get_properties(&instance, physical_device) };

        let buffer_device_address = BufferDeviceAddress::new(&instance, &device);

        let context_raytracing = ContextRaytracing {
            ray_tracing_pipeline,
            physical_device_ray_tracing_pipeline_properties_khr,
            acceleration_structure,
            physical_device_acceleration_structure_properties_khr,
        };

        let device_memory_properties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };

        Self {
            _entry: entry,
            instance,

            surface,
            surface_loader,

            context_raytracing,
            synchronisation2_loader,

            physical_device,
            queue_family_index,

            device,
            queue,
            buffer_device_address,
            device_memory_properties,
        }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe { self.device.destroy_device(None) };

        unsafe { self.surface_loader.destroy_surface(self.surface, None) };

        unsafe { self.instance.destroy_instance(None) };
    }
}

fn find_physical_device(
    instance: &ash::Instance,
    surface: &vk::SurfaceKHR,
    surface_loader: &ash::extensions::khr::Surface,
) -> (vk::PhysicalDevice, u32) {
    let swapchain_extension = ash::extensions::khr::Swapchain::name();

    let (physical_device, queue_family_index) = {
        let physical_devices = unsafe { instance.enumerate_physical_devices() }
            .expect("Could not enumerate physical devices");

        physical_devices
            .into_iter()
            .filter(|pd| {
                let extension_properties =
                    unsafe { instance.enumerate_device_extension_properties(*pd) }
                        .expect("Could not enumerate device extension properties");
                let mut supported_extensions = extension_properties
                    .iter()
                    .map(|property| unsafe { CStr::from_ptr(property.extension_name.as_ptr()) });

                supported_extensions.any(|ext| swapchain_extension == ext)
            })
            .filter_map(|pd| {
                unsafe { instance.get_physical_device_queue_family_properties(pd) }
                    .iter()
                    .enumerate()
                    .position(|(index, info)| {
                        let supports_graphics = info.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                        let supports_surface = unsafe {
                            surface_loader.get_physical_device_surface_support(
                                pd,
                                index as u32,
                                *surface,
                            )
                        }
                        .unwrap();

                        supports_graphics && supports_surface
                    })
                    .map(|i| (pd, i as u32))
            })
            .min_by_key(|(pd, _)| {
                let device_type =
                    unsafe { instance.get_physical_device_properties(*pd) }.device_type;

                match device_type {
                    vk::PhysicalDeviceType::DISCRETE_GPU => 0,
                    vk::PhysicalDeviceType::INTEGRATED_GPU => 1,
                    vk::PhysicalDeviceType::VIRTUAL_GPU => 2,
                    vk::PhysicalDeviceType::CPU => 3,
                    vk::PhysicalDeviceType::OTHER => 4,
                    _ => 5,
                }
            })
            .expect("Couldn't find suitable device.")
    };

    (physical_device, queue_family_index)
}

fn create_logical_device(
    instance: &ash::Instance,
    physical_device: &vk::PhysicalDevice,
) -> ash::Device {
    let swapchain_extension = ash::extensions::khr::Swapchain::name();
    let synchronisation2_extension = ash::extensions::khr::Synchronization2::name();
    let acceleration_structure_extension = ash::extensions::khr::AccelerationStructure::name();
    let ray_tracing_pipeline_extension = ash::extensions::khr::RayTracingPipeline::name();
    let deferred_host_operations_extension = ash::extensions::khr::DeferredHostOperations::name();
    let device_address_extension = ash::extensions::khr::BufferDeviceAddress::name();

    let device_extensions = [
        swapchain_extension.as_ptr(),
        synchronisation2_extension.as_ptr(),
        acceleration_structure_extension.as_ptr(),
        ray_tracing_pipeline_extension.as_ptr(),
        deferred_host_operations_extension.as_ptr(),
        device_address_extension.as_ptr(),
    ];

    let queue_priorities = [1.0];
    let queue_create_info = DeviceQueueCreateInfo::builder()
        .queue_family_index(0)
        .queue_priorities(&queue_priorities);

    let mut physical_device_vulkan13_features = vk::PhysicalDeviceVulkan13Features {
        synchronization2: vk::TRUE,
        ..vk::PhysicalDeviceVulkan13Features::default()
    };

    let mut enabled_buffer_device_address_features =
        vk::PhysicalDeviceBufferDeviceAddressFeatures {
            buffer_device_address: vk::TRUE,
            ..vk::PhysicalDeviceBufferDeviceAddressFeatures::default()
        };

    let mut enabled_ray_tracing_pipeline_features =
        vk::PhysicalDeviceRayTracingPipelineFeaturesKHR {
            ray_tracing_pipeline: vk::TRUE,
            ..vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default()
        };

    let mut enabled_acceleration_structure_features =
        vk::PhysicalDeviceAccelerationStructureFeaturesKHR {
            acceleration_structure: vk::TRUE,
            ..vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default()
        };

    let create_info = DeviceCreateInfo::builder()
        .queue_create_infos(std::slice::from_ref(&queue_create_info))
        .enabled_extension_names(&device_extensions)
        .push_next(&mut physical_device_vulkan13_features)
        .push_next(&mut enabled_buffer_device_address_features)
        .push_next(&mut enabled_ray_tracing_pipeline_features)
        .push_next(&mut enabled_acceleration_structure_features)
        .build();

    unsafe { instance.create_device(*physical_device, &create_info, None) }
        .expect("Could not create logical device")
}
