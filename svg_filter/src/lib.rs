use std::error::Error;
use std::ffi::{c_void, CStr};
use std::os::raw::{c_char, c_double, c_int, c_uchar, c_uint};
use std::ptr;

lazy_static::lazy_static! {
    pub(crate) static ref RENDER_OPTIONS: resvg_cairo::Options = resvg_cairo::Options {
        usvg: usvg::Options {
            shape_rendering: usvg::ShapeRendering::GeometricPrecision,
            image_rendering: usvg::ImageRendering::OptimizeQuality,
            text_rendering: usvg::TextRendering::GeometricPrecision,
            ..usvg::Options::default()
        },
        fit_to: usvg::FitTo::Original,
        background: None,
    };
}

#[no_mangle]
pub extern "C" fn filter_init(config: *const c_char, user_data: *mut *mut c_void) -> c_int {
    unsafe {
        *user_data = ptr::null_mut();
    }

    if config.is_null() {
        eprintln!("got null config");
        return 1;
    }

    let svg_path = match parse_config(config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error parsing config: {}", e);
            return 1;
        }
    };

    let tree = match usvg::Tree::from_file(svg_path, &RENDER_OPTIONS.usvg) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error reading svg: {}", e);
            return 1;
        }
    };

    unsafe {
        *user_data = Box::into_raw(Box::new(tree)) as *mut c_void;
    }
    0
}

#[no_mangle]
pub extern "C" fn filter_frame(
    data: *mut c_uchar,
    data_size: c_uint,
    width: c_int,
    height: c_int,
    line_size: c_int,
    _ts_millis: c_double,
    user_data: *mut c_void,
) -> c_int {
    let cr = match new_cairo_context(data, data_size as usize, width, height, line_size) {
        Ok(cr) => cr,
        Err(status) => {
            eprintln!("could not create cairo context: {}", status);
            return 1;
        }
    };

    let tree = if user_data.is_null() {
        eprintln!("no user data");
        return 1;
    } else {
        unsafe { &*(user_data as *const usvg::Tree) }
    };

    let size = usvg::ScreenSize::new(width as u32, height as u32).unwrap();
    resvg_cairo::render_to_canvas(tree, &RENDER_OPTIONS, size, &cr);

    0
}

#[no_mangle]
pub extern "C" fn filter_version(_ts_millis: c_double, _user_data: *mut c_void) -> u64 {
    1
}

#[no_mangle]
pub extern "C" fn filter_uninit(user_data: *mut c_void) {
    if !user_data.is_null() {
        unsafe {
            drop(Box::from_raw(user_data as *mut usvg::Tree));
        }
    }
}

fn parse_config(config: *const c_char) -> Result<String, Box<dyn Error>> {
    let regex = regex::Regex::new(r"^svg=(.*)$")?;
    let opt = unsafe { CStr::from_ptr(config) }.to_str()?;
    let captures = regex.captures(opt).ok_or("invalid option, use: svg=path")?;
    let path = captures.get(1).unwrap().as_str();
    Ok(String::from(path))
}

fn new_cairo_context(
    data: *mut c_uchar,
    _data_size: usize,
    width: i32,
    height: i32,
    line_size: i32,
) -> Result<cairo::Context, cairo::Status> {
    let surface = unsafe {
        let surface = cairo_sys::cairo_image_surface_create_for_data(
            data,
            cairo_sys::FORMAT_A_RGB32,
            width,
            height,
            line_size,
        );

        cairo::ImageSurface::from_raw_full(surface)?
    };

    let cr = cairo::Context::new(&surface);
    cr.set_antialias(cairo::Antialias::Gray);
    cr.set_tolerance(0.01);
    Ok(cr)
}
