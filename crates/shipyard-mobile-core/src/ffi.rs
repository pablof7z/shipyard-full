use std::os::raw::c_char;

pub const CAPABILITY_DEVICE_TOKENS: u64 = 1 << 0;
pub const CAPABILITY_QUEUE_PREVIEW: u64 = 1 << 1;
pub const CAPABILITY_SIGNED_SCHEDULING: u64 = 1 << 2;
pub const CAPABILITY_PROPOSALS: u64 = 1 << 3;
pub const CAPABILITY_NIP37_DRAFTS: u64 = 1 << 4;
pub const CAPABILITY_BLOSSOM_SELECTION: u64 = 1 << 5;

const VERSION: &[u8] = b"0.1.0\0";
const CAPABILITY_NAMES: [&[u8]; 6] = [
    b"device_tokens\0",
    b"queue_preview\0",
    b"signed_scheduling\0",
    b"proposals\0",
    b"nip37_drafts\0",
    b"blossom_selection\0",
];

#[no_mangle]
pub extern "C" fn shipyard_mobile_core_version_major() -> u32 {
    0
}

#[no_mangle]
pub extern "C" fn shipyard_mobile_core_version_minor() -> u32 {
    1
}

#[no_mangle]
pub extern "C" fn shipyard_mobile_core_version_patch() -> u32 {
    0
}

#[no_mangle]
pub extern "C" fn shipyard_mobile_core_version() -> *const c_char {
    VERSION.as_ptr().cast()
}

#[no_mangle]
pub extern "C" fn shipyard_mobile_core_capabilities() -> u64 {
    CAPABILITY_DEVICE_TOKENS
        | CAPABILITY_QUEUE_PREVIEW
        | CAPABILITY_SIGNED_SCHEDULING
        | CAPABILITY_PROPOSALS
        | CAPABILITY_NIP37_DRAFTS
        | CAPABILITY_BLOSSOM_SELECTION
}

#[no_mangle]
pub extern "C" fn shipyard_mobile_core_capability_count() -> u32 {
    CAPABILITY_NAMES.len() as u32
}

#[no_mangle]
pub extern "C" fn shipyard_mobile_core_capability_name(index: u32) -> *const c_char {
    CAPABILITY_NAMES
        .get(index as usize)
        .map_or(std::ptr::null(), |name| name.as_ptr().cast())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn exposes_version_and_capabilities_for_ffi() {
        assert_eq!(shipyard_mobile_core_version_major(), 0);
        assert_eq!(shipyard_mobile_core_version_minor(), 1);
        assert_ne!(shipyard_mobile_core_capabilities(), 0);

        let name = unsafe { CStr::from_ptr(shipyard_mobile_core_capability_name(4)) };
        assert_eq!(name.to_str().unwrap(), "nip37_drafts");
        assert!(shipyard_mobile_core_capability_name(99).is_null());
    }
}
