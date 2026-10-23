use bevy::reflect::{FromReflect, GetTypeRegistration, Reflect, Typed};

mod sealed {
    pub trait Sealed {}
}

pub trait WidgetryNumber:
    sealed::Sealed
    + Copy
    + Default
    + PartialOrd
    + Reflect
    + FromReflect
    + GetTypeRegistration
    + Typed
    + std::fmt::Display
    + std::str::FromStr
{
    const ZERO: Self;
    const ONE: Self;
    const FLOAT: bool;
    const SIGNED: bool;
    fn is_finite(self) -> bool;
    fn parse_decimal(text: &str) -> Option<Self> {
        if !plain_decimal(text, Self::FLOAT, Self::SIGNED) {
            return None;
        }
        let value = text.parse::<Self>().ok()?;
        value.is_finite().then_some(value)
    }
    fn canonical(self) -> String {
        let mut text = self.to_string();
        if Self::FLOAT && !text.contains('.') {
            text.push_str(".0");
        }
        text
    }
}

fn plain_decimal(text: &str, float: bool, signed: bool) -> bool {
    let text = if signed {
        text.strip_prefix('-').unwrap_or(text)
    } else {
        text
    };
    let mut digit = false;
    let mut dot = false;
    for byte in text.bytes() {
        if byte.is_ascii_digit() {
            digit = true;
        } else if float && byte == b'.' && !dot {
            dot = true;
        } else {
            return false;
        }
    }
    digit
}

macro_rules! integers {
    ($signed:expr; $($t:ty),+ $(,)?) => {$(
        impl sealed::Sealed for $t {}
        impl WidgetryNumber for $t {
            const ZERO: Self = 0;
            const ONE: Self = 1;
            const FLOAT: bool = false;
            const SIGNED: bool = $signed;
            fn is_finite(self) -> bool { true }
        }
    )+};
}
integers!(true; i8, i16, i32, i64, i128, isize);
integers!(false; u8, u16, u32, u64, u128, usize);

macro_rules! floats {
    ($($t:ty),+ $(,)?) => {$(
        impl sealed::Sealed for $t {}
        impl WidgetryNumber for $t {
            const ZERO: Self = 0.0;
            const ONE: Self = 1.0;
            const FLOAT: bool = true;
            const SIGNED: bool = true;
            fn is_finite(self) -> bool { self.is_finite() }
        }
    )+};
}
floats!(f32, f64);

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn decimal_grammar_keeps_integer_sign_and_type_ranges() {
        assert_eq!(i128::parse_decimal(&i128::MIN.to_string()), Some(i128::MIN));
        assert_eq!(u128::parse_decimal(&u128::MAX.to_string()), Some(u128::MAX));
        assert_eq!(i8::parse_decimal("128"), None);
        assert_eq!(u8::parse_decimal("256"), None);
        for text in [
            "", "-", "-0", "-1", "1.0", "1e2", "0xff", "1,000", " 1", "+1",
        ] {
            assert_eq!(u32::parse_decimal(text), None, "{text}");
        }
        for text in ["1.0", "1e2", "-", "", "1,000"] {
            assert_eq!(i32::parse_decimal(text), None, "{text}");
        }
        assert_eq!(i32::parse_decimal("-123"), Some(-123));
        for text in ["NaN", "Infinity", "inf", "1e3", ".1.2", ".", "-.", "-", ""] {
            assert_eq!(f32::parse_decimal(text), None, "{text}");
            assert_eq!(f64::parse_decimal(text), None, "{text}");
        }
    }
}
