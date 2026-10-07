//! Single audited Win32 call: bind an opened pipe handle to the selected acad.exe PID.
use std::os::windows::io::AsRawHandle;
use tokio::net::windows::named_pipe::NamedPipeClient;
pub fn server_is_pid(pipe: &NamedPipeClient, expected: u32) -> bool {
    let mut actual = 0u32;
    // SAFETY: pipe owns a live Windows handle; Win32 writes one u32 to `actual`.
    let ok = unsafe {
        windows_sys::Win32::System::Pipes::GetNamedPipeServerProcessId(
            pipe.as_raw_handle(), &mut actual)
    };
    ok != 0 && actual == expected
}
