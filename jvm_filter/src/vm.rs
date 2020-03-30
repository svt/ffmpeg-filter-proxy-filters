// SPDX-FileCopyrightText: 2020 Sveriges Television AB
//
// SPDX-License-Identifier: Apache-2.0

use std::ptr;

use jni_sys;
use libc::c_void;

use crate::init_args::InitArgs;

pub struct VM {
    pub vm: *mut jni_sys::JavaVM,
    pub env: *mut jni_sys::JNIEnv,
}

impl VM {
    pub fn new(init_args: InitArgs) -> Option<VM> {
        let mut vm: *mut jni_sys::JavaVM = ptr::null_mut();
        let mut env: *mut jni_sys::JNIEnv = ptr::null_mut();
        let rv = unsafe {
            jni_sys::JNI_CreateJavaVM(
                &mut vm as *mut _,
                &mut env as *mut *mut jni_sys::JNIEnv as *mut *mut c_void,
                init_args.inner_ptr(),
            )
        };

        if rv == jni_sys::JNI_OK {
            Some(VM { vm, env })
        } else {
            None
        }
    }

    pub fn is_ok(&self) -> bool {
        unsafe {
            !(self.vm.is_null()
                || (*self.vm).is_null()
                || self.env.is_null()
                || (*self.env).is_null())
        }
    }

    pub fn is_compatible(&self) -> bool {
        let (vm, env) = unsafe { (**self.vm, **self.env) };
        vm.DetachCurrentThread.is_some()
            && vm.DestroyJavaVM.is_some()
            && env.ExceptionCheck.is_some()
            && env.ExceptionDescribe.is_some()
            && env.ExceptionClear.is_some()
            && env.FindClass.is_some()
            && env.GetStaticMethodID.is_some()
            && env.CallStaticVoidMethod.is_some()
            && env.CallStaticObjectMethod.is_some()
            && env.NewByteArray.is_some()
            && env.SetByteArrayRegion.is_some()
            && env.GetArrayLength.is_some()
            && env.GetByteArrayElements.is_some()
            && env.ReleaseByteArrayElements.is_some()
            && env.DeleteLocalRef.is_some()
    }

    pub fn exception_thrown(&self) -> bool {
        unsafe {
            let env = **self.env;
            if env.ExceptionCheck.unwrap()(self.env) == jni_sys::JNI_TRUE {
                env.ExceptionDescribe.unwrap()(self.env);
                env.ExceptionClear.unwrap()(self.env);
                true
            } else {
                false
            }
        }
    }
}

impl Drop for VM {
    fn drop(&mut self) {
        unsafe {
            if !self.vm.is_null() && !(*self.vm).is_null() {
                let vm = **self.vm;
                vm.DetachCurrentThread.unwrap()(self.vm);
                vm.DestroyJavaVM.unwrap()(self.vm);
            }
        }
    }
}
