use std::ffi::{c_char, c_int};
use std::os::raw::c_void;
use std::{fs, io, mem};

const EMBEDDED_LIB: &[u8] = include_bytes!("libddwaf-1.27.0_arm64.dylib.gz");

unsafe extern "C" {
    #[link_name = "dlopen"]
    unsafe fn dlopen(path: *const c_char, mode: c_int) -> *mut c_void;

    #[cfg(feature = "dlsym")]
    #[link_name = "dlsym"]
    unsafe fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;

    #[cfg(feature = "dlclose")]
    #[link_name = "dlclose"]
    unsafe fn dlclose(handle: *mut c_void) -> c_int;
}

const RTLD_NOW: c_int = 0x2;
const RTLD_LOCAL: c_int = 0x4;

pub fn main() {
    for idx in 0..100 {
        #[cfg(feature = "verbose")]
        println!("Iteration {idx}");
        #[cfg(not(feature = "verbose"))]
        let _ = idx;

        let tmpdir = tempfile::tempdir().expect("Failed to create temporary directory");
        let tmpfile = tmpdir.path().join("libddwaf.dylib");
        let mut decoder = flate2::read::GzDecoder::new(EMBEDDED_LIB);
        let mut file = fs::File::create(&tmpfile).expect("Failed to create file");
        io::copy(&mut decoder, &mut file).expect("Failed to write decompressed dylib");
        mem::drop(file);

        let handle = unsafe {
            dlopen(
                tmpfile.to_string_lossy().as_ptr().cast(),
                RTLD_LOCAL | RTLD_NOW,
            )
        };
        debug_assert!(!handle.is_null());

        #[cfg(feature = "dlsym")]
        {
            use std::ffi::CStr;

            let symbol = b"ddwaf_get_version\0";
            let ptr = unsafe { dlsym(handle, symbol.as_ptr().cast()) };
            debug_assert!(!ptr.is_null());
            let callable: extern "C" fn() -> *const c_char = unsafe { mem::transmute(ptr) };
            let version = callable();
            assert_eq!("1.27.0", unsafe { CStr::from_ptr(version) }.to_string_lossy())
        }

        #[cfg(feature = "dlclose")]
        {
            let result = unsafe { dlclose(handle) };
            debug_assert_eq!(result, 0);
        }

        fs::remove_file(&tmpfile).expect("Failed to remove file");
        tmpdir
            .close()
            .expect("Failed to clean up temporary directory");
    }

    println!("All done, did not crash!");
}
