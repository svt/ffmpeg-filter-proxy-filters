#[macro_use]
extern crate lazy_static;
extern crate regex;

use std::error::Error;
use std::ffi::{c_void, CStr};
use std::os::raw::{c_char, c_double, c_int, c_uchar, c_uint};
use std::ptr;

use resvg::{cairo, usvg};


lazy_static! {
    pub(crate) static ref RESVG_OPTIONS: resvg::Options = resvg::Options {
        usvg: usvg::Options {
            shape_rendering: usvg::ShapeRendering::GeometricPrecision,
            image_rendering: usvg::ImageRendering::OptimizeQuality,
            text_rendering: usvg::TextRendering::GeometricPrecision,
            ..usvg::Options::default()
        },
        ..resvg::Options::default()
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
            eprintln!("Error parsing config: {}", e);
            return 1;
        }
    };

    let tree = match usvg::Tree::from_file(svg_path, &RESVG_OPTIONS.usvg) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Error reading svg: {}", e);
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
            eprintln!("Could not create cairo context: {}", status);
            return 1;
        }
    };

    let tree = if user_data.is_null() {
        eprintln!("no user data");
        return 1;
    } else {
        let t = unsafe { Box::from_raw(user_data as *mut usvg::Tree) };
        Box::leak(t)
    };

    let size = resvg::ScreenSize::new(width as u32, height as u32).unwrap();
    resvg::backend_cairo::render_to_canvas(&tree, &RESVG_OPTIONS, size, &cr);

    0
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
    let captures = regex.captures(opt).ok_or("Invalid option! use: svg=path")?;
    let path = captures.get(1).unwrap().as_str();
    Ok(String::from(path))
}

fn new_cairo_context(
    data: *mut c_uchar,
    data_size: usize,
    width: i32,
    height: i32,
    line_size: i32,
) -> Result<cairo::Context, cairo::Status> {
    let data = unsafe { std::slice::from_raw_parts_mut(data, data_size) };
    let surface = cairo::ImageSurface::create_for_data(
        data,
        cairo::Format::ARgb32,
        width,
        height,
        line_size,
    )?;

    let cr = cairo::Context::new(&surface);
    cr.set_antialias(cairo::Antialias::Best);
    Ok(cr)
}

