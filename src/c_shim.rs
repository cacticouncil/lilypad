use std::{
    alloc::{self, Layout},
    ffi::{c_int, c_void},
    mem::align_of,
    ptr,
};

/* -------------------------------- stdlib.h -------------------------------- */

#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    if size == 0 {
        return ptr::null_mut();
    }

    let (layout, offset_to_data) = layout_for_size_prepended(size);
    let buf = alloc::alloc(layout);
    store_layout(buf, layout, offset_to_data)
}

#[no_mangle]
pub unsafe extern "C" fn calloc(count: usize, size: usize) -> *mut c_void {
    if count == 0 || size == 0 {
        return ptr::null_mut();
    }

    let (layout, offset_to_data) = layout_for_size_prepended(size * count);
    let buf = alloc::alloc_zeroed(layout);
    store_layout(buf, layout, offset_to_data)
}

#[no_mangle]
pub unsafe extern "C" fn realloc(buf: *mut c_void, new_size: usize) -> *mut c_void {
    if buf.is_null() {
        malloc(new_size)
    } else if new_size == 0 {
        free(buf);
        ptr::null_mut()
    } else {
        let (old_buf, old_layout) = retrieve_layout(buf);
        let (new_layout, offset_to_data) = layout_for_size_prepended(new_size);
        let new_buf = alloc::realloc(old_buf, old_layout, new_layout.size());
        store_layout(new_buf, new_layout, offset_to_data)
    }
}

#[no_mangle]
pub unsafe extern "C" fn free(buf: *mut c_void) {
    if buf.is_null() {
        return;
    }
    let (buf, layout) = retrieve_layout(buf);
    alloc::dealloc(buf, layout);
}

// In all these allocations, we store the layout before the data for later retrieval.
// This is because we need to know the layout when deallocating the memory.
// Here are some helper methods for that:

/// Given a pointer to the data, retrieve the layout and the pointer to the layout.
unsafe fn retrieve_layout(buf: *mut c_void) -> (*mut u8, Layout) {
    let (_, layout_offset) = Layout::new::<Layout>()
        .extend(Layout::from_size_align(0, align_of::<*const u8>() * 2).unwrap())
        .unwrap();

    let buf = (buf as *mut u8).offset(-(layout_offset as isize));
    let layout = *(buf as *mut Layout);

    (buf, layout)
}

/// Calculate a layout for a given size with space for storing a layout at the start.
/// Returns the layout and the offset to the data.
fn layout_for_size_prepended(size: usize) -> (Layout, usize) {
    Layout::new::<Layout>()
        .extend(Layout::from_size_align(size, align_of::<*const u8>() * 2).unwrap())
        .unwrap()
}

/// Store a layout in the pointer, returning a pointer to where the data should be stored.
unsafe fn store_layout(buf: *mut u8, layout: Layout, offset_to_data: usize) -> *mut c_void {
    *(buf as *mut Layout) = layout;
    (buf as *mut u8).offset(offset_to_data as isize) as *mut c_void
}

/* -------------------------------- string.h -------------------------------- */

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut c_void, src: *const c_void, size: usize) -> *mut c_void {
    std::ptr::copy_nonoverlapping(src, dest, size);
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memmove(
    dest: *mut c_void,
    src: *const c_void,
    size: usize,
) -> *mut c_void {
    std::ptr::copy(src, dest, size);
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut c_void, c: i32, n: usize) -> *mut c_void {
    let slice = std::slice::from_raw_parts_mut(s as *mut u8, n);
    slice.fill(c as u8);
    s
}

/* -------------------------------- wctype.h -------------------------------- */

#[no_mangle]
pub unsafe extern "C" fn iswspace(c: c_int) -> bool {
    char::from_u32(c as u32).map_or(false, |c| c.is_whitespace())
}
