extern crate libc;
#[macro_use] extern crate appinstance;
extern crate ferrite;
extern crate ws_common;
extern crate metrics;

use ferrite as fe;
use ws_common::*;
use metrics::*;

struct StartInfo { pub metrics: Size2 }
impl Default for StartInfo { fn default() -> Self { StartInfo { metrics: Size2(960, 540) } } }
impl StartInfo
{
    fn parse_opt() -> Self
    {
        let mut info = Self::default();

        if let Some(n) = std::env::args().position(|e| e == "-size")
        {
            let size_str = std::env::args().nth(n + 1).expect("Specify size(format: wxh)");
            let size_spl: Vec<_> = size_str.split("x").collect();
            let (w, h) = (size_spl[0].parse().expect("Required an integer"), size_spl[1].parse().expect("Required an integer"));
            info.metrics = Size2(w, h);
        }
        info
    }
}

#[cfg(windows)] const PLATFORM_SURFACE_LAYER: &'static str = "VK_KHR_win32_surface";
#[cfg(feature = "target_x11")] const PLATFORM_SURFACE_LAYER: &'static str = "VK_KHR_xcb_surface";
struct Renderer
{
    instance: fe::Instance, adapter: fe::PhysicalDevice, device: fe::Device, queue: (fe::Queue, u32),
    dbg: fe::DebugReportCallback
}
impl Renderer
{
    AppInstance!(static instance: Renderer = Renderer::init());

    fn init() -> Self
    {
        let instance = fe::InstanceBuilder::new("screenshader", (0, 1, 0), "ShaderSandbox", (0, 1, 0))
            .add_layer("VK_LAYER_LUNARG_standard_validation")
            .add_extensions(vec!["VK_KHR_surface", PLATFORM_SURFACE_LAYER, "VK_EXT_debug_report"]).create().expect("Failed to initialize Vulkan");
        let dbg = fe::DebugReportCallback::new::<()>(&instance, fe::DebugReportFlags::ERROR.warning().performance_warning(), Self::debug_callback, None)
            .expect("Failed to create debug object");
        let adapter = instance.enumerate_physical_devices().expect("Failed to enumerate physical devices").remove(0);
        let qinfo = adapter.queue_family_properties();
        let gqf = qinfo.find_matching_index(fe::QueueFlags::GRAPHICS).expect("Failed to find a family index of graphics queue");
        let device = fe::DeviceBuilder::new(&adapter)
            .add_layer("VK_LAYER_LUNARG_standard_validation").add_extension("VK_KHR_swapchain")
            .add_queue(fe::DeviceQueueCreateInfo(gqf, vec![0.0])).create().expect("Failed to create Device");
        let queue = device.queue(gqf, 0);
        Renderer { instance, adapter, device, queue: (queue, gqf), dbg }
    }
    fn make_surface(&self, n: &NativeWindow) -> fe::Surface
    {
        if !WindowServer::instance().presentation_support(&self.adapter, self.queue.1)
        {
            panic!("xcb is not support Vulkan presentation");
        }
        let s = WindowServer::instance().new_render_surface(n, &self.instance).expect("Failed to create Surface");
        if !self.adapter.surface_support(self.queue.1, &s).expect("Failed to query whether the adapter is support surface")
        {
            panic!("Adapter is not support this surface");
        }
        s
    }

    extern "system" fn debug_callback(_flags: fe::vk::VkDebugReportFlagsEXT, _object_type: fe::vk::VkDebugReportObjectTypeEXT, _object: u64,
        _location: libc::size_t, _message_code: i32, _layer_prefix: *const libc::c_char, message: *const libc::c_char, _user_data: *mut libc::c_void) -> fe::vk::VkBool32
    {
        let message = unsafe { std::ffi::CStr::from_ptr(message).to_string_lossy() };
        println!("SysDebug: {:?}", message); false as _
    }
}

fn main()
{
    let sinfo = StartInfo::parse_opt();
    let mainwnd = NativeWindow::new((sinfo.metrics.width() as _, sinfo.metrics.height() as _), "ScreenShader", false);
    let _surface = Renderer::instance().make_surface(&mainwnd);
    mainwnd.show();

    WindowServer::instance().process_events();
}
