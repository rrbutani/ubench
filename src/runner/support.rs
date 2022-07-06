#[derive(Debug, Copy)]
#[doc(hidden)]
pub struct HListIterator<'a, Inner: ?Sized>(pub &'a Inner);

impl<'a, T: ?Sized> Clone for HListIterator<'a, T> {
    fn clone(&self) -> Self {
        Self(<&T>::clone(&self.0))
    }
}

pub fn black_box<T>(x: T) -> T {
    // TODO: inline asm method??
    unsafe {
        let ret = core::ptr::read_volatile(&x);
        core::mem::forget(x);
        ret
    }
}
