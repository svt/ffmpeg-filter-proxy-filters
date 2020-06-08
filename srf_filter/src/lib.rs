// SPDX-FileCopyrightText: 2020 Sveriges Television AB
//
// SPDX-License-Identifier: Apache-2.0

use std::ffi::CStr;
use std::fs::File;
use std::ptr;

use libc::{c_char, c_double, c_int, c_uchar, c_uint, c_void};

mod subtitle_rendering_data;
use subtitle_rendering_data::{Point, RenderingData, SegmentType, Transition};

enum ScaleType {
    None,
    Uniform,
    NonUniform,
}

impl Default for ScaleType {
    fn default() -> Self {
        Self::Uniform
    }
}

struct Config<'a> {
    scale_type: ScaleType,
    srf: &'a str,
}

struct Context {
    scale_type: ScaleType,
    rendering_data: RenderingData,
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

    let config = match parse_config(config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("invalid config: {:?}", e);
            return 1;
        }
    };

    let rendering_data = match read_srf(config.srf) {
        Ok(rd) => rd,
        Err(e) => {
            eprintln!("{}: {:?}", config.srf, e);
            return 1;
        }
    };

    let ctx = Context {
        scale_type: config.scale_type,
        rendering_data,
    };

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
        unsafe { &*(user_data as *const Context) }
    };

    let transitions = ctx.rendering_data.get_transitions();
    let transition = match find_transition(transitions, ts_millis) {
        Some(idx) => &transitions[idx],
        None => {
            return 0;
        }
    };

    let cr = match new_cairo_context(data, data_size as usize, width, height, line_size) {
        Ok(cr) => cr,
        Err(status) => {
            eprintln!("could not create cairo context: {:?}", status);
            return 1;
        }
    };

    let render_ctx = RenderContext {
        ctx: &ctx,
        transition: &transition,
        cr: &cr,
    };

    render_ctx.scale(width as f64, height as f64);
    render_ctx.render_shapes();

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

fn parse_config<'a>(config: *const c_char) -> anyhow::Result<Config<'a>> {
    let cstr = unsafe { CStr::from_ptr(config) };
    let s = cstr.to_str()?;
    let re = regex::Regex::new(r"^(?:scale_type=(none|uniform|non_uniform),)?srf=(.+)$").unwrap();
    if let Some(cap) = re.captures(s) {
        let scale_type = if let Some(st) = cap.get(1) {
            match st.as_str() {
                "none" => ScaleType::None,
                "uniform" => ScaleType::Uniform,
                _ => ScaleType::NonUniform,
            }
        } else {
            ScaleType::Uniform
        };

        Ok(Config {
            scale_type,
            srf: cap.get(2).unwrap().as_str(),
        })
    } else {
        Err(anyhow::anyhow!(s))
    }
}

fn read_srf(srf: &str) -> anyhow::Result<RenderingData> {
    let mut f = File::open(srf)?;
    let rendering_data = protobuf::parse_from_reader(&mut f)?;
    Ok(rendering_data)
}

fn find_transition(transitions: &[Transition], ts: f64) -> Option<usize> {
    transitions
        .binary_search_by(|t| {
            let time_in = t.get_time_in() as f64;
            let time_out = t.get_time_out() as f64;
            if ts >= time_in && ts < time_out {
                std::cmp::Ordering::Equal
            } else if time_in < ts || time_out < ts {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        })
        .ok()
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
    cr.set_antialias(cairo::Antialias::Best);
    Ok(cr)
}

const SAFE_AR_WIDTH: f64 = 11.0;
const SAFE_AR_HEIGHT: f64 = 10.0;
const SAFE_AR: f64 = SAFE_AR_WIDTH / SAFE_AR_HEIGHT;

struct RenderContext<'a> {
    ctx: &'a Context,
    transition: &'a Transition,
    cr: &'a cairo::Context,
}

impl<'a> RenderContext<'a> {
    fn scale(&self, width: f64, height: f64) {
        if let ScaleType::None = self.ctx.scale_type {
            return;
        }

        let rd_width = self.ctx.rendering_data.get_width() as f64;
        let rd_height = self.ctx.rendering_data.get_height() as f64;
        if let ScaleType::NonUniform = self.ctx.scale_type {
            self.cr.scale(width / rd_width, height / rd_height);
            return;
        }

        let (sx, sy) = if width / height >= SAFE_AR {
            // scale to height.
            let sy = height / rd_height;
            (sy, sy)
        } else {
            // scale to safe AR width.
            let sx = width * SAFE_AR_HEIGHT / SAFE_AR_WIDTH / rd_height;
            (sx, sx)
        };

        self.cr.scale(sx, sy);

        let (rd_width, rd_height) = self.cr.user_to_device(rd_width, rd_height);
        let tx = if rd_width < width {
            // translate to center.
            (width - rd_width) / 2.
        } else if rd_width > width {
            // translate left resulting in crop to
            // max(ratio safe, pic aspect ratio).
            -((rd_width - (SAFE_AR_HEIGHT * rd_height / SAFE_AR_WIDTH)) / 2.0)
                .min((rd_width - width) / 2.0)
        } else {
            0.
        };

        let ty = if rd_height < height {
            // translate to bottom.
            height - rd_height
        } else {
            0.
        };

        let (tx, ty) = self.cr.device_to_user(tx, ty);
        self.cr.translate(tx, ty);
    }

    fn render_shapes(&self) {
        for shape in self.transition.get_shapes() {
            self.cr.save();
            self.cr
                .translate(unfix(shape.get_x()), unfix(shape.get_y()));

            let path = &self.ctx.rendering_data.get_paths()[shape.get_path_index() as usize];
            for seg in path.get_segments() {
                match seg.get_field_type() {
                    SegmentType::MOVE => {
                        let p = &seg.get_points()[0];
                        self.cr.move_to(unfix(p.get_x()), unfix(p.get_y()));
                    }
                    SegmentType::LINE => {
                        let p = &seg.get_points()[0];
                        self.cr.line_to(unfix(p.get_x()), unfix(p.get_y()));
                    }
                    SegmentType::QUAD => {
                        let c = &seg.get_points()[0];
                        let p = &seg.get_points()[1];
                        self.quad_to_curve(c, p);
                    }
                    SegmentType::CUBIC => {
                        let c1 = &seg.get_points()[0];
                        let c2 = &seg.get_points()[1];
                        let p = &seg.get_points()[2];
                        self.cr.curve_to(
                            unfix(c1.get_x()),
                            unfix(c1.get_y()),
                            unfix(c2.get_x()),
                            unfix(c2.get_y()),
                            unfix(p.get_x()),
                            unfix(p.get_y()),
                        );
                    }
                    SegmentType::CLOSE => {
                        self.cr.close_path();
                    }
                };
            }

            self.set_color(shape.get_argb());
            if shape.get_fill() {
                self.cr.fill();
            } else {
                self.cr.set_line_cap(cairo::LineCap::Square);
                self.cr.set_line_join(cairo::LineJoin::Round);
                self.cr.set_line_width(shape.get_line_width() as f64 / 64.);
                self.cr.stroke();
            }

            self.cr.restore();
        }
    }

    fn quad_to_curve(&self, c: &Point, p: &Point) {
        let (x1, y1) = self.cr.get_current_point();

        let x2 = unfix(c.get_x());
        let y2 = unfix(c.get_y());
        let x3 = unfix(p.get_x());
        let y3 = unfix(p.get_y());
        self.cr.curve_to(
            x1 + (2. / 3.) * (x2 - x1),
            y1 + (2. / 3.) * (y2 - y1),
            x3 + (2. / 3.) * (x2 - x3),
            y3 + (2. / 3.) * (y2 - y3),
            x3,
            y3,
        );
    }

    fn set_color(&self, argb: u32) {
        let r = ((argb >> 16) & 0xFF) as f64 / 255.;
        let g = ((argb >> 8) & 0xFF) as f64 / 255.;
        let b = ((argb >> 0) & 0xFF) as f64 / 255.;
        let a = ((argb >> 24) & 0xFF) as f64 / 255.;
        self.cr.set_source_rgba(r, g, b, a);
    }
}

#[inline]
fn unfix(n: i32) -> f64 {
    return n as f64 / 64.;
}
