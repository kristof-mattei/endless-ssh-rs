use libc::c_int;
use libc::c_void;
use libc::setsockopt;
use libc::socklen_t;
use libc::SOL_SOCKET;
use libc::SO_RCVBUF;
use std::io::Error;
use std::mem::size_of_val;
use std::net::TcpStream;
use std::os::unix::prelude::AsRawFd;
use std::ptr::addr_of;

pub(crate) fn set_receive_buffer_size(
    tcp_stream: &TcpStream,
    size_in_bytes: usize,
) -> Result<(), Error> {
    // Set the smallest possible recieve buffer. This reduces local
    // resource usage and slows down the remote end.
    let value: i32 = i32::try_from(size_in_bytes).expect("Byte buffer didn't fit in an i32");

    #[allow(clippy::cast_possible_truncation)]
    let r: c_int = unsafe {
        setsockopt(
            tcp_stream.as_raw_fd(),
            SOL_SOCKET,
            SO_RCVBUF,
            addr_of!(value).cast::<c_void>(),
            size_of_val(&value) as socklen_t,
        )
    };

    if r == -1 {
        return Err(Error::last_os_error());
    }

    Ok(())
}
