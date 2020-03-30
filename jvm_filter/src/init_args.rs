// SPDX-FileCopyrightText: 2020 Sveriges Television AB
//
// SPDX-License-Identifier: Apache-2.0

use std::ffi::{CString, NulError};
use std::ptr;

use jni_sys::{JavaVMInitArgs, JavaVMOption, JNI_VERSION_1_8};
use libc::c_void;

pub struct InitArgsBuilder(Vec<String>);

impl Default for InitArgsBuilder {
    fn default() -> Self {
        InitArgsBuilder(Vec::new())
    }
}

impl InitArgsBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn property(&mut self, key: &str, val: &str) {
        self.0.push(format!("-D{}={}", key, val));
    }

    pub fn build(self) -> Result<InitArgs, NulError> {
        let mut opts = Vec::with_capacity(self.0.len());
        for opt in self.0 {
            let opt_str = CString::new(opt.as_str())?;
            let jvm_opt = JavaVMOption {
                optionString: opt_str.into_raw(),
                extraInfo: ptr::null_mut(),
            };

            opts.push(jvm_opt);
        }

        let init_args = InitArgs {
            inner: JavaVMInitArgs {
                version: JNI_VERSION_1_8,
                nOptions: opts.len() as _,
                options: opts.as_ptr() as _,
                ignoreUnrecognized: 0,
            },
            opts,
        };

        Ok(init_args)
    }
}

pub struct InitArgs {
    inner: JavaVMInitArgs,
    opts: Vec<JavaVMOption>,
}

impl Drop for InitArgs {
    fn drop(&mut self) {
        for opt in self.opts.iter() {
            unsafe {
                CString::from_raw(opt.optionString);
            }
        }
    }
}

impl InitArgs {
    pub fn inner_ptr(&self) -> *mut c_void {
        &self.inner as *const _ as _
    }
}
