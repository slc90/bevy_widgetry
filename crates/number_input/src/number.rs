use bevy::reflect::{FromReflect, GetTypeRegistration, Reflect, Typed};

mod sealed {
    pub trait Sealed {}
}

pub trait WidgetryNumber:
    sealed::Sealed + Copy + Default + PartialOrd + Reflect + FromReflect + GetTypeRegistration + Typed
{
    const ZERO: Self;
    const ONE: Self;
    fn is_finite(self) -> bool;
}

macro_rules! integers {
    ($($t:ty),+ $(,)?) => {$(
        impl sealed::Sealed for $t {}
        impl WidgetryNumber for $t {
            const ZERO: Self = 0;
            const ONE: Self = 1;
            fn is_finite(self) -> bool { true }
        }
    )+};
}
integers!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

macro_rules! floats {
    ($($t:ty),+ $(,)?) => {$(
        impl sealed::Sealed for $t {}
        impl WidgetryNumber for $t {
            const ZERO: Self = 0.0;
            const ONE: Self = 1.0;
            fn is_finite(self) -> bool { self.is_finite() }
        }
    )+};
}
floats!(f32, f64);
