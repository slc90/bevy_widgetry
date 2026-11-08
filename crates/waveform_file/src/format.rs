use std::ops::Range;

pub(crate) const MAGIC: &[u8; 8] = b"WFRIDX02";
pub(crate) const RAW_DATA_START: u64 = MAGIC.len() as u64;
pub(crate) const FLOAT_FIELD_LEN: usize = 4;
pub(crate) const FOOTER_LEN: usize = 16;
pub(crate) const FOOTER_HEADER_SIZE: Range<usize> = 0..8;
pub(crate) const FOOTER_MAGIC: Range<usize> = 8..16;
pub(crate) const MIN_FILE_LEN: u64 = RAW_DATA_START + FOOTER_LEN as u64;
pub(crate) const MAX_HEADER_LEN: u64 = 8_388_608;
pub(crate) const UNIT_VOLTS: u8 = 0;
pub(crate) const UNIT_MILLIVOLTS: u8 = 1;
pub(crate) const UNIT_MICROVOLTS: u8 = 2;

pub(crate) mod raw {
    use std::ops::Range;
    pub(crate) const LEN: usize = 5;
    pub(crate) const VALUE: Range<usize> = 0..4;
    pub(crate) const VALIDITY: usize = 4;
    pub(crate) const INVALID: u8 = 0;
    pub(crate) const VALID: u8 = 1;
}

pub(crate) mod span {
    use std::ops::Range;
    pub(crate) const LEN: usize = 24;
    pub(crate) const START: Range<usize> = 0..8;
    pub(crate) const END: Range<usize> = 8..16;
    pub(crate) const MIN: Range<usize> = 16..20;
    pub(crate) const MAX: Range<usize> = 20..24;
}

pub(crate) mod index {
    use std::ops::Range;
    pub(crate) const LEN: usize = 16;
    pub(crate) const OFFSET_LEN: usize = 8;
    pub(crate) const OFFSET: Range<usize> = 0..8;
    pub(crate) const COUNT: Range<usize> = 8..16;
}

pub(crate) mod point {
    use std::ops::Range;
    pub(crate) const LEN: usize = 16;
    pub(crate) const SAMPLE: Range<usize> = 0..8;
    pub(crate) const VALUE: Range<usize> = 8..12;
    pub(crate) const BREAK_BEFORE: Range<usize> = 12..16;
    pub(crate) const BREAK_FLAG: u32 = 1;
}
