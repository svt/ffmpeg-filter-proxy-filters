// SPDX-FileCopyrightText: 2020 Sveriges Television AB
//
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::ptr;

use jni_sys;
use libc::{c_char, c_double, c_int, c_uchar, c_uint, c_void};
use serde::Deserialize;

mod init_args;
use init_args::InitArgsBuilder;

mod vm;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Config<'a> {
    class_name: &'a str,
    properties: HashMap<&'a str, &'a str>,
}

struct Context {
    vm: vm::VM,
    class: jni_sys::jclass,
    on_frame_method: jni_sys::jmethodID,
    destroy_method: jni_sys::jmethodID,
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

    let class_name = match config.class_name.trim() {
        "" => {
            eprintln!("empty class_name in config");
            return 1;
        }
        cn => cn,
    };

    let mut init_args_builder = InitArgsBuilder::new();
    for (k, v) in &config.properties {
        let k = k.trim();
        let v = v.trim();
        if k.is_empty() || v.is_empty() {
            eprintln!("empty property key and/or value in config");
            return 1;
        }

        init_args_builder.property(k, v);
    }

    let init_args = match init_args_builder.build() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("error building JVM init args: {:?}", e);
            return 1;
        }
    };

    let vm = match vm::VM::new(init_args) {
        Some(vm) => {
            if !vm.is_ok() {
                eprintln!("invalid JVM state");
                return 1;
            } else if !vm.is_compatible() {
                eprintln!("incompatible JVM");
                return 1;
            }

            vm
        }

        None => {
            eprintln!("could not create JVM");
            return 1;
        }
    };

    let class = unsafe {
        let c = CString::new(class_name).unwrap();
        let env = **vm.env;
        env.FindClass.unwrap()(vm.env, c.as_ptr())
    };

    if vm.exception_thrown() || class.is_null() {
        eprintln!("could not find class {}", class_name);
        return 1;
    }

    let init_method = resolve_method(vm.env, class, "init", "()V");
    if vm.exception_thrown() || init_method.is_null() {
        eprintln!("could not resolve init method");
        return 1;
    }

    let on_frame_method = resolve_method(vm.env, class, "onFrame", "([BIIID)[B");
    if vm.exception_thrown() || init_method.is_null() {
        eprintln!("could not resolve onFrame method");
        return 1;
    }

    let destroy_method = resolve_method(vm.env, class, "destroy", "()V");
    if vm.exception_thrown() || init_method.is_null() {
        eprintln!("could not resolve destroy method");
        return 1;
    }

    unsafe {
        (*(*vm.env)).CallStaticVoidMethod.unwrap()(vm.env, class, init_method);
        if vm.exception_thrown() {
            eprintln!("error calling init method");
            return 1;
        }
    }

    let ctx = Context {
        vm,
        class,
        on_frame_method,
        destroy_method,
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

    if !ctx.vm.is_ok() {
        eprintln!("invalid JVM state");
        return 1;
    }

    let in_arr = unsafe { (*(*ctx.vm.env)).NewByteArray.unwrap()(ctx.vm.env, data_size as _) };
    if ctx.vm.exception_thrown() || in_arr.is_null() {
        eprintln!("could not create byte array");
        return 1;
    }

    unsafe {
        let env = **ctx.vm.env;
        env.SetByteArrayRegion.unwrap()(ctx.vm.env, in_arr, 0, data_size as _, data as _);
        if ctx.vm.exception_thrown() {
            eprintln!("could not set byte array region");
            env.DeleteLocalRef.unwrap()(ctx.vm.env, in_arr);
            return 1;
        }
    }

    let out_arr = unsafe {
        (*(*ctx.vm.env)).CallStaticObjectMethod.unwrap()(
            ctx.vm.env,
            ctx.class,
            ctx.on_frame_method,
            in_arr,
            width,
            height,
            line_size,
            ts_millis,
        )
    };

    let err = ctx.vm.exception_thrown();
    unsafe {
        (*(*ctx.vm.env)).DeleteLocalRef.unwrap()(ctx.vm.env, in_arr);
    }

    if err {
        eprintln!("error calling onFrame method");
        return 1;
    }

    if !out_arr.is_null() {
        unsafe {
            let env = **ctx.vm.env;
            let out_arr_len = env.GetArrayLength.unwrap()(ctx.vm.env, out_arr);
            if out_arr_len != data_size as i32 {
                eprintln!(
                    "onFrame returned a byte array with invalid length: {} != {}",
                    out_arr_len, data_size as i32
                );

                env.DeleteLocalRef.unwrap()(ctx.vm.env, out_arr);
                return 1;
            }

            let is_copy = ptr::null_mut();
            let buf = env.GetByteArrayElements.unwrap()(ctx.vm.env, out_arr, is_copy);
            if buf.is_null() {
                eprintln!("null array elements");
                env.DeleteLocalRef.unwrap()(ctx.vm.env, out_arr);
                return 1;
            }

            ptr::copy_nonoverlapping(buf, data as _, data_size as _);
            env.ReleaseByteArrayElements.unwrap()(ctx.vm.env, out_arr, buf, 0);
            env.DeleteLocalRef.unwrap()(ctx.vm.env, out_arr);
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn filter_uninit(user_data: *mut c_void) {
    if !user_data.is_null() {
        unsafe {
            let ctx = Box::from_raw(user_data as *mut Context);
            if ctx.vm.is_ok() {
                (*(*ctx.vm.env)).CallStaticVoidMethod.unwrap()(
                    ctx.vm.env,
                    ctx.class,
                    ctx.destroy_method,
                );
            }
        }
    }
}

fn parse_config<'a>(config: *const c_char) -> Result<Config<'a>, Box<dyn std::error::Error>> {
    let cstr = unsafe { CStr::from_ptr(config) };
    let s = cstr.to_str()?;
    let c = serde_json::from_str(s)?;
    Ok(c)
}

fn resolve_method(
    env: *mut jni_sys::JNIEnv,
    class: jni_sys::jclass,
    name: &str,
    sig: &str,
) -> jni_sys::jmethodID {
    let n = CString::new(name).unwrap();
    let s = CString::new(sig).unwrap();
    unsafe { (*(*env)).GetStaticMethodID.unwrap()(env, class, n.as_ptr(), s.as_ptr()) }
}
