use core::fmt;

// Unifies `core::fmt::Write`, `embedded-hal::serial::Write`, `std::io::Write`.
pub trait Output {
    fn output_str(&mut self, s: &str);
    fn output_char(&mut self, c: char) {
        self.output_str(c.encode_utf8(&mut [0; 4]))
    }
    fn output_fmt(&mut self, args: fmt::Arguments<'_>) {
        let mut s = OutputAdapter(self);
        fmt::write(&mut s, args).unwrap()
    }
    fn flush(&mut self) {}
}

pub struct Void;
impl Output for Void {
    fn output_str(&mut self, _s: &str) {}
    fn output_fmt(&mut self, _args: fmt::Arguments<'_>) {}
}

impl Output for dyn fmt::Write + '_ {
    #[inline(always)]
    fn output_str(&mut self, s: &str) {
        self.write_str(s).unwrap();
    }

    #[inline(always)]
    fn output_char(&mut self, c: char) {
        self.write_char(c).unwrap()
    }

    fn output_fmt(&mut self, args: fmt::Arguments<'_>) {
        self.write_fmt(args).unwrap()
    }
}

#[cfg(feature = "std")]
#[cfg_attr(all(docs, not(doctest)), doc(cfg(feature = "std")))]
impl Output for dyn std::io::Write + '_ {
    fn output_str(&mut self, s: &str) {
        self.write_all(s.as_bytes()).unwrap();
    }

    fn flush(&mut self) {
        std::io::Write::flush(self).unwrap()
    }
}

// duplicate of the impl in `embedded_hal` except this one doesn't require `+ 'static`
#[cfg(feature = "embedded-hal")]
#[cfg_attr(all(docs, not(doctest)), doc(cfg(feature = "embedded-hal")))]
impl<W: embedded_hal::serial::Write<u8>> Output for W {
    fn output_str(&mut self, s: &str) {
        s.as_bytes()
            .iter()
            .for_each(|c| nb::block!(self.write(*c)).map_err(|_| ()).unwrap());
    }

    fn flush(&mut self) {
        W::flush(self).map_err(|_| ()).unwrap();
    }
}

pub(super) struct OutputAdapter<'o, O: ?Sized + Output = dyn Output>(pub(super) &'o mut O);

impl<'o, O: ?Sized + Output> fmt::Write for OutputAdapter<'o, O> {
    #[inline(always)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.output_str(s);
        Ok(())
    }

    #[inline(always)]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.0.output_char(c);
        Ok(())
    }

    #[inline(always)]
    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.0.output_fmt(args);
        Ok(())
    }
}
