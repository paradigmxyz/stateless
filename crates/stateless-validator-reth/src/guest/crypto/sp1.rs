#[unsafe(no_mangle)]
extern "C" fn native_keccak256(bytes: *const u8, len: usize, output: *mut u8) {
    let mut hash = zkvm_interface::zkvm_keccak256_hash { data: [0; 32] };
    unsafe {
        zkvm_interface::zkvm_keccak256(bytes, len, &mut hash);
        core::ptr::copy_nonoverlapping(hash.data.as_ptr(), output, 32);
    }
}
