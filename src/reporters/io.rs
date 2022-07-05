use core::fmt;

// Unifies `core::fmt::Write`, `embedded-hal::serial::Write`, `std::io::Write`.
pub trait Output {
    fn output_str(&mut self, s: &str);
    fn output_char(&mut self, c: char) {
        self.output_str(c.encode_utf8(&mut [0; 4]))
    }
    fn output_fmt(&mut self, args: fmt::Arguments<'_>);
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

    fn output_fmt(&mut self, args: fmt::Arguments<'_>) {
        // let mut s: &mut (dyn std::io::Write + 'a) = self;
        // let mut s: &mut (dyn Output + 'a) = &mut (s as (dyn std::io::Write + 'a));
        // let s: &mut (dyn fmt::Write + 'a) = &mut s as _;

        // can't seem to get rustc to let us cast from `&mut (dyn std::io::Write
        // + 'a)` to `&mut (dyn Output + '_)`..
        //
        // so we introduce this wrapper type and yet _another_ level of
        // indirection (that LLVM will hopefully boil away)

        struct StdIoWriteNewType<'o>(&'o mut dyn std::io::Write);
        impl<'o> Output for StdIoWriteNewType<'o> {
            #[inline(always)]
            fn output_str(&mut self, s: &str) { self.0.output_str(s) }
            #[inline(always)]
            fn output_char(&mut self, c: char) { self.0.output_char(c) }
            #[inline(always)]
            fn output_fmt(&mut self, args: fmt::Arguments<'_>) { self.0.output_fmt(args) }

            fn flush(&mut self) { unreachable!() }
        }

        let mut s = StdIoWriteNewType::<'_>(self);
        let mut s: &mut (dyn Output + '_) = &mut s as &mut (dyn Output + '_);
        let s: &mut (dyn fmt::Write + '_) = &mut s as _;

        fmt::write(s, args).unwrap()
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

    fn output_fmt<'a>(&'a mut self, args: fmt::Arguments<'_>) {
        let mut s: &mut (dyn Output + 'a) = self as &mut (dyn Output + 'a);
        let s: &mut (dyn fmt::Write + 'a) = &mut s as _;

        fmt::write(s, args).unwrap()
    }

    fn flush(&mut self) {
        W::flush(self).map_err(|_| ()).unwrap();
    }
}

impl fmt::Write for dyn Output + '_ {
    #[inline(always)]
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.output_str(s);
        Ok(())
    }

    #[inline(always)]
    fn write_char(&mut self, c: char) -> fmt::Result {
        self.output_char(c);
        Ok(())
    }
}
