use std::alloc::{self, Layout};
use std::os::raw::c_void;

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut c_void, src: *const c_void, size: usize) -> *mut c_void {
    let dest_slice = std::slice::from_raw_parts_mut(dest as *mut u8, size);
    let src_slice = std::slice::from_raw_parts(src as *const u8, size);
    dest_slice.copy_from_slice(src_slice);
    dest
}

#[no_mangle]
pub unsafe extern "C" fn calloc(count: usize, size: usize) -> *mut u8 {
    let layout = Layout::from_size_align_unchecked(count * size, std::mem::align_of::<u8>());
    alloc::alloc(layout)
}

#[no_mangle]
pub unsafe extern "C" fn realloc(ptr: *mut u8, new_size: usize) -> *mut u8 {
    let layout = Layout::from_size_align_unchecked(new_size, std::mem::align_of::<u8>());
    alloc::realloc(ptr, layout, new_size)
}

#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut u8) {
    let layout = Layout::from_size_align_unchecked(0, std::mem::align_of::<u8>());
    alloc::dealloc(ptr, layout);
}

#[no_mangle]
pub unsafe extern "C" fn iswspace(c: char) -> bool {
    c.is_whitespace()
}