// SPDX-FileCopyrightText: 2020 Sveriges Television AB
//
// SPDX-License-Identifier: Apache-2.0

use std::ffi::CStr;
use std::fs::File;
use std::ptr;

use flate2::read::GzDecoder;
use libc::{c_char, c_double, c_int, c_uchar, c_uint, c_void};
use regex::Regex;
use resvg::{cairo, usvg};
use simple_error::SimpleError;

use lazy_static::lazy_static;

mod parse;

mod transition;
use transition::Tree;

pub(crate) type BoxResult<T> = Result<T, Box<dyn std::error::Error>>;

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

#[derive(Debug)]
enum Compression {
    None,
    Gzip,
}

struct Config<'a> {
    compression: Compression,
    tsvg: &'a str,
}

struct Context(Tree);

#[no_mangle]
pub extern "C" fn filter_init(config: *const c_char, user_data: *mut *mut c_void) -> c_int {
    unsafe {
        *user_data = ptr::null_mut();
    }

    if config.is_null() {
        eprintln!("got null config");
        return 1;
    }

    let tree = match parse_config(config).and_then(|c| parse_tsvg(&c)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error parsing: {}", e);
            return 1;
        }
    };

    let ctx = Context(tree);
    unsafe {
        *user_data = Box::into_raw(Box::new(ctx)) as *mut c_void;
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
    ts_millis: c_double,
    user_data: *mut c_void,
) -> c_int {
    if data.is_null() || width <= 0 || height <= 0 {
        return 0;
    }

    let ctx = if user_data.is_null() {
        eprintln!("no user data");
        return 1;
    } else {
        let ctx = unsafe { Box::from_raw(user_data as *mut Context) };
        Box::leak(ctx)
    };

    let transitions = ctx.0.search(ts_millis);
    if transitions.is_empty() {
        return 0;
    }

    let cr = match new_cairo_context(data, data_size as usize, width, height, line_size) {
        Ok(cr) => cr,
        Err(status) => {
            eprintln!("could not create cairo context: {:?}", status);
            return 1;
        }
    };

    let size = resvg::ScreenSize::new(width as u32, height as u32).unwrap();
    for transition in transitions {
        resvg::backend_cairo::render_to_canvas(&transition.tree, &RESVG_OPTIONS, size, &cr);
    }

    0
}

#[no_mangle]
pub extern "C" fn filter_uninit(user_data: *mut c_void) {
    if !user_data.is_null() {
        unsafe {
            drop(Box::from_raw(user_data as *mut Context));
        }
    }
}

fn parse_config<'a>(config: *const c_char) -> BoxResult<Config<'a>> {
    let cstr = unsafe { CStr::from_ptr(config) };
    let s = cstr.to_str()?;
    let re = Regex::new(r"^(?:compression=(none|gzip),)?tsvg=(.+)$").unwrap();
    if let Some(cap) = re.captures(s) {
        let compression = if let Some(c) = cap.get(1) {
            if c.as_str() == "none" {
                Compression::None
            } else {
                Compression::Gzip
            }
        } else {
            Compression::Gzip
        };

        Ok(Config {
            compression,
            tsvg: cap.get(2).unwrap().as_str(),
        })
    } else {
        Err(SimpleError::new(s).into())
    }
}

fn parse_tsvg(config: &Config) -> BoxResult<Tree> {
    let f = File::open(config.tsvg)?;
    if let Compression::Gzip = config.compression {
        parse::parse_tsvg(GzDecoder::new(f))
    } else {
        parse::parse_tsvg(f)
    }
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
