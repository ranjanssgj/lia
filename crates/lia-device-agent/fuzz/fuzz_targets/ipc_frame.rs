#![no_main]

use libfuzzer_sys::fuzz_target;
use lia_device_agent::ipc::read_frame;
use std::io::Cursor;

fuzz_target!(|data: &[u8]| {
    if let Ok(envelope) = read_frame(&mut Cursor::new(data)) {
        envelope
            .validate()
            .expect("frame decoder returned an invalid envelope");
    }
});
