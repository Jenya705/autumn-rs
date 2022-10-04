use std::ptr::NonNull;

pub(crate) struct UnknownPointer {
    // As pointer type is unknown in static, we should call it destructor on side
    destructor: fn(*mut ()),
    pointer: NonNull<()>,
}

impl UnknownPointer {
    pub fn new<T>(pointer: NonNull<T>) -> Self {
        Self {
            pointer: pointer.cast(),
            // Safety. We are casting fn(*mut T) to the fn(*mut ()) what is the same.
            destructor: unsafe { std::mem::transmute(std::ptr::drop_in_place::<T> as *mut ()) }
        }
    }

    pub fn get(&self) -> &NonNull<()> {
        &self.pointer
    }
}

impl Drop for UnknownPointer {
    fn drop(&mut self) {
        let pointer = self.pointer.as_ptr();
        (self.destructor)(pointer);
        drop(unsafe { Box::from_raw(pointer) })
    }
}