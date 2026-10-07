use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, ReadBuf};
/// Bound each newline-delimited MCP JSON frame before the SDK accumulates it.
pub struct BoundedRead<R> {
    inner: R,
    line_bytes: usize,
}
impl<R> BoundedRead<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            line_bytes: 0,
        }
    }
}
impl<R: AsyncRead + Unpin> AsyncRead for BoundedRead<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let previous = buf.filled().len();
        match Pin::new(&mut this.inner).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                for b in &buf.filled()[previous..] {
                    if *b == b'\n' {
                        this.line_bytes = 0;
                    } else {
                        this.line_bytes += 1;
                        if this.line_bytes > 1024 * 1024 {
                            buf.set_filled(previous);
                            return Poll::Ready(Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "MCP frame exceeds limit",
                            )));
                        }
                    }
                }
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}
