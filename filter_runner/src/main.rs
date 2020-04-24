// SPDX-FileCopyrightText: 2020 Sveriges Television AB
//
// SPDX-License-Identifier: Apache-2.0

use std::ffi::CString;
use std::fs::File;
use std::io::BufWriter;
use std::ptr;

use clap::{value_t_or_exit, App, Arg};
use dlopen::wrapper::{Container, WrapperApi};
use dlopen_derive::*;
use libc::{c_char, c_double, c_int, c_uchar, c_uint, c_void};
use png;

#[derive(WrapperApi)]
struct FilterApi {
    filter_init: unsafe extern "C" fn(config: *const c_char, user_data: *mut *mut c_void) -> c_int,
    filter_frame: unsafe extern "C" fn(
        data: *mut c_uchar,
        data_size: c_uint,
        width: c_int,
        height: c_int,
        line_size: c_int,
        ts_millis: c_double,
        user_data: *mut c_void,
    ) -> c_int,
    filter_uninit: unsafe extern "C" fn(user_data: *mut c_void),
}

fn main() {
    let matches = App::new("Filter Runner")
        .arg(
            Arg::with_name("FILTER")
                .help("Sets the filter to run")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .help("Sets the config")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("timestamp")
                .short("t")
                .long("timestamp")
                .help("Sets the timestamp")
                .takes_value(true)
                .default_value("0"),
        )
        .arg(
            Arg::with_name("width")
                .short("w")
                .long("width")
                .help("Sets the frame width")
                .takes_value(true)
                .default_value("1280"),
        )
        .arg(
            Arg::with_name("height")
                .short("h")
                .long("height")
                .help("Sets the frame height")
                .takes_value(true)
                .default_value("720"),
        )
        .arg(
            Arg::with_name("png_out")
                .short("o")
                .long("png_out")
                .help("Sets the PNG output file")
                .takes_value(true),
        )
        .get_matches();

    let ts = value_t_or_exit!(matches.value_of("timestamp"), c_double);
    let width = value_t_or_exit!(matches.value_of("width"), c_int);
    let height = value_t_or_exit!(matches.value_of("height"), c_int);
    let filter = matches.value_of("FILTER").unwrap();
    let config = matches.value_of("config").unwrap_or("");
    let png_out = matches.value_of("png_out").unwrap_or("");

    let container: Container<FilterApi> = unsafe { Container::load(filter) }.unwrap();

    let mut user_data: *mut c_void = ptr::null_mut();
    let rv = unsafe {
        let cfg = CString::new(config).unwrap();
        container.filter_init(cfg.as_ptr(), &mut user_data)
    };

    println!("filter_init returned {}", rv);
    if rv != 0 {
        std::process::exit(rv);
    }

    let line_size: c_int = width * 4;

    let mut frame_data: Vec<u8> = vec![0x55; (height * line_size) as _];
    let rv = unsafe {
        container.filter_frame(
            frame_data.as_mut_ptr(),
            frame_data.len() as _,
            width,
            height,
            line_size,
            ts,
            user_data,
        )
    };

    println!("filter_frame returned {}", rv);
    unsafe {
        container.filter_uninit(user_data);
    }

    if rv == 0 && png_out != "" {
        println!("writing PNG to {}", png_out);

        let file = File::create(png_out).unwrap();
        let ref mut w = BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, width as _, height as _);
        encoder.set_color(png::ColorType::RGBA);
        encoder.set_depth(png::BitDepth::Eight);

        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&frame_data).unwrap();
    }
}
