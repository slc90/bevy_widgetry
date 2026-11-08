const BITMAP_BITS_PER_PIXEL: u16 = 32;
const VERBATIM_DISK_PREFIX_UNITS: usize = 4;
const VERBATIM_UNC_PREFIX_UNITS: usize = 8;
const UNC_PREFIX: [u16; 2] = [92, 92];

use super::{IconKey, Pixels, SIZE};
use std::{io, mem::size_of, os::windows::ffi::OsStrExt, ptr};
use windows_sys::Win32::{
    Graphics::Gdi::*,
    Storage::FileSystem::{FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_NORMAL},
    System::Com::{
        COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE, CoInitializeEx, CoUninitialize,
    },
    UI::{Shell::*, WindowsAndMessaging::*},
};

pub(super) struct Apartment(std::marker::PhantomData<std::rc::Rc<()>>);

impl Apartment {
    pub(super) fn new() -> io::Result<Self> {
        // SAFETY: Every batch calls this on its own fresh managed OS thread.
        let result = unsafe {
            CoInitializeEx(
                ptr::null(),
                (COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE) as u32,
            )
        };
        if result < 0 {
            return Err(io::Error::other(format!(
                "CoInitializeEx HRESULT {result:#x}"
            )));
        }
        Ok(Self(std::marker::PhantomData))
    }
}

impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: The same worker owns one successful initialization, including S_FALSE.
        unsafe { CoUninitialize() };
    }
}

struct Icon(HICON);

impl Drop for Icon {
    fn drop(&mut self) {
        // SAFETY: SHGFI_ICON transfers this unique icon handle to the caller.
        unsafe { DestroyIcon(self.0) };
    }
}

struct Surface {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    pixels: *mut u8,
}

impl Surface {
    fn new() -> io::Result<Self> {
        // SAFETY: A null compatible DC creates a private memory DC on this thread.
        let dc = unsafe { CreateCompatibleDC(ptr::null_mut()) };
        if dc.is_null() {
            return Err(io::Error::last_os_error());
        }
        let mut surface = Self {
            dc,
            bitmap: ptr::null_mut(),
            previous: ptr::null_mut(),
            pixels: ptr::null_mut(),
        };
        // SAFETY: BITMAPINFO is plain Win32 data; all unused fields must be zero.
        let mut info: BITMAPINFO = unsafe { std::mem::zeroed() };
        info.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = SIZE as i32;
        info.bmiHeader.biHeight = -(SIZE as i32);
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = BITMAP_BITS_PER_PIXEL;
        info.bmiHeader.biCompression = BI_RGB;
        // SAFETY: The header describes a fixed-size top-down 32-bit DIB and pixels is writable.
        surface.bitmap = unsafe {
            CreateDIBSection(
                dc,
                &info,
                DIB_RGB_COLORS,
                (&mut surface.pixels as *mut *mut u8).cast(),
                ptr::null_mut(),
                0,
            )
        };
        if surface.bitmap.is_null() || surface.pixels.is_null() {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: This exclusively owned bitmap is not selected into any other DC.
        surface.previous = unsafe { SelectObject(dc, surface.bitmap) };
        if surface.previous.is_null() || surface.previous as isize == -1 {
            surface.previous = ptr::null_mut();
            return Err(io::Error::last_os_error());
        }
        Ok(surface)
    }

    fn draw(&mut self, icon: &Icon, background: u8) -> io::Result<Vec<u8>> {
        let length = (SIZE * SIZE * super::RGBA_CHANNELS as u32) as usize;
        // SAFETY: CreateDIBSection allocated exactly this fixed BGRA byte region.
        let bytes = unsafe { std::slice::from_raw_parts_mut(self.pixels, length) };
        for pixel in bytes.as_chunks_mut::<{ super::RGBA_CHANNELS }>().0 {
            pixel.copy_from_slice(&[background, background, background, super::OPAQUE_ALPHA]);
        }
        // SAFETY: Both handles are live on this thread; the target DIB has SIZE by SIZE pixels.
        if unsafe {
            DrawIconEx(
                self.dc,
                0,
                0,
                icon.0,
                SIZE as i32,
                SIZE as i32,
                0,
                ptr::null_mut(),
                DI_NORMAL,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: Flush this thread's GDI work before directly reading the DIB allocation.
        if unsafe { GdiFlush() } == 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: The flush completed all writes and the surface still owns this allocation.
        Ok(unsafe { std::slice::from_raw_parts(self.pixels, length) }.to_vec())
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        // SAFETY: Restore the borrowed stock object before releasing our bitmap, then the DC.
        unsafe {
            if !self.previous.is_null() {
                SelectObject(self.dc, self.previous);
            }
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap);
            }
            DeleteDC(self.dc);
        }
    }
}

fn shell_path(path: &std::ffi::OsStr) -> Vec<u16> {
    use std::path::{Component, Prefix};
    let mut wide = path.encode_wide().collect::<Vec<_>>();
    if let Some(Component::Prefix(prefix)) = std::path::Path::new(path).components().next() {
        match prefix.kind() {
            // Shell parsing does not accept the verbatim disk/UNC syntax returned by canonicalize.
            // Convert only the query representation, preserving every UTF-16 path unit and the cache key.
            Prefix::VerbatimDisk(_) => {
                wide.drain(..VERBATIM_DISK_PREFIX_UNITS);
            }
            Prefix::VerbatimUNC(_, _) => {
                wide.splice(..VERBATIM_UNC_PREFIX_UNITS, UNC_PREFIX);
            }
            _ => {}
        }
    }
    wide
}

pub(super) fn extract(key: &IconKey) -> io::Result<Option<Pixels>> {
    let (path, attributes, extra) = match key {
        IconKey::Folder => (
            std::ffi::OsString::from("folder"),
            FILE_ATTRIBUTE_DIRECTORY,
            SHGFI_USEFILEATTRIBUTES,
        ),
        IconKey::FileType(extension) => {
            let mut path = std::ffi::OsString::from("file");
            if !extension.is_empty() {
                path.push(".");
                path.push(extension);
            }
            (path, FILE_ATTRIBUTE_NORMAL, SHGFI_USEFILEATTRIBUTES)
        }
        IconKey::Path(path) => (path.as_os_str().to_owned(), 0, 0),
    };
    let mut wide = shell_path(&path);
    if wide.contains(&0) {
        return Ok(None);
    }
    wide.push(0);
    // SAFETY: This is an output-only POD structure with its complete size supplied below.
    let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
    // SAFETY: The UTF-16 input and SHFILEINFOW output remain live for the synchronous call.
    // No overlay flags or thumbnail APIs are requested.
    let found = unsafe {
        SHGetFileInfoW(
            wide.as_ptr(),
            attributes,
            &mut info,
            size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON | extra,
        )
    };
    if found == 0 || info.hIcon.is_null() {
        // SHGetFileInfo does not expose a reliable GetLastError contract for unavailable icons.
        return Ok(None);
    }
    let icon = Icon(info.hIcon);
    let mut surface = Surface::new()?;
    let black = surface.draw(&icon, 0)?;
    let white = surface.draw(&icon, super::OPAQUE_ALPHA)?;
    Ok(Some(Pixels(super::straight_rgba(&black, &white))))
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use windows_sys::Win32::System::Threading::{
        GR_GDIOBJECTS, GR_USEROBJECTS, GetCurrentProcess, GetGuiResources,
    };

    fn counts() -> (u32, u32) {
        // SAFETY: The pseudo process handle is borrowed and both flags are valid counters.
        unsafe {
            (
                GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS),
                GetGuiResources(GetCurrentProcess(), GR_USEROBJECTS),
            )
        }
    }

    #[test]
    fn shell_query_paths_preserve_utf16_for_disk_unc_and_other_namespaces() {
        use std::os::windows::ffi::OsStringExt;
        for (source, expected) in [
            (r"\\?\C:\fixture\icon.ico", r"C:\fixture\icon.ico"),
            (r"\\?\UNC\server\share\icon.ico", r"\\server\share\icon.ico"),
            (r"\\?\Volume{test}\icon.ico", r"\\?\Volume{test}\icon.ico"),
        ] {
            assert_eq!(
                shell_path(std::ffi::OsStr::new(source)),
                expected.encode_utf16().collect::<Vec<_>>()
            );
        }
        let mut raw = r"\\?\C:\fixture\".encode_utf16().collect::<Vec<_>>();
        raw.push(0xd800);
        raw.extend(".ico".encode_utf16());
        assert_eq!(shell_path(&std::ffi::OsString::from_wide(&raw)), raw[4..]);
    }

    #[test]
    fn repeated_extraction_does_not_accumulate_gdi_or_icon_handles() {
        const CHILD: &str = "WIDGETRY_SHELL_HANDLE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let output=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","icons::native::tests::repeated_extraction_does_not_accumulate_gdi_or_icon_handles","--nocapture"]).env(CHILD,"1").output().unwrap();
            assert!(
                output.status.success(),
                "{} {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let _apartment = Apartment::new().unwrap();
        let key = IconKey::FileType("txt".into());
        for _ in 0..10 {
            extract(&key).unwrap().unwrap();
        }
        let before = counts();
        for _ in 0..100 {
            extract(&key).unwrap().unwrap();
        }
        assert_eq!(counts(), before);
    }

    #[test]
    fn ico_files_keep_their_own_pixels_instead_of_the_extension_default() {
        fn write_icon(path: &std::path::Path, bgra: [u8; 4]) {
            const BITMAP_HEADER_BYTES: u32 = 40;
            const BGRA_BYTES_PER_PIXEL: u32 = 4;
            const MASK_BITS_PER_BYTE: u32 = 8;
            const ICON_KIND: u16 = 1;
            const IMAGE_COUNT: u16 = 1;
            const COLOR_PLANES: u16 = 1;
            const COLOR_BITS_PER_PIXEL: u16 = 32;
            const BITMAP_DATA_OFFSET: u32 = 22;
            const MASK_HEIGHT_FACTOR: u32 = 2;
            let payload = BITMAP_HEADER_BYTES
                + SIZE * SIZE * BGRA_BYTES_PER_PIXEL
                + SIZE * SIZE / MASK_BITS_PER_BYTE;
            let mut bytes = Vec::new();
            for value in [0u16, ICON_KIND, IMAGE_COUNT] {
                bytes.extend(value.to_le_bytes());
            }
            bytes.extend([SIZE as u8, SIZE as u8, 0, 0]);
            bytes.extend(COLOR_PLANES.to_le_bytes());
            bytes.extend(COLOR_BITS_PER_PIXEL.to_le_bytes());
            bytes.extend(payload.to_le_bytes());
            bytes.extend(BITMAP_DATA_OFFSET.to_le_bytes());
            for value in [BITMAP_HEADER_BYTES, SIZE, SIZE * MASK_HEIGHT_FACTOR] {
                bytes.extend(value.to_le_bytes());
            }
            bytes.extend(COLOR_PLANES.to_le_bytes());
            bytes.extend(COLOR_BITS_PER_PIXEL.to_le_bytes());
            for value in [0u32, SIZE * SIZE * BGRA_BYTES_PER_PIXEL, 0, 0, 0, 0] {
                bytes.extend(value.to_le_bytes());
            }
            for _ in 0..SIZE * SIZE {
                bytes.extend(bgra);
            }
            bytes.resize(bytes.len() + (SIZE * SIZE / MASK_BITS_PER_BYTE) as usize, 0);
            std::fs::write(path, bytes).unwrap();
        }
        std::thread::spawn(|| {
            let _apartment = Apartment::new().unwrap();
            let directory = tempfile::tempdir().unwrap();
            let red = directory.path().join("red.ico");
            let blue = directory.path().join("blue.ico");
            write_icon(&red, [0, 0, 255, 255]);
            write_icon(&blue, [255, 0, 0, 255]);
            let Pixels(red) = extract(&IconKey::Path(std::fs::canonicalize(red).unwrap()))
                .unwrap()
                .unwrap();
            let Pixels(blue) = extract(&IconKey::Path(blue)).unwrap().unwrap();
            assert!(
                red.as_chunks::<4>()
                    .0
                    .iter()
                    .any(|pixel| pixel[0] > 240 && pixel[2] < 10 && pixel[3] == 255)
            );
            assert!(
                blue.as_chunks::<4>()
                    .0
                    .iter()
                    .any(|pixel| pixel[2] > 240 && pixel[0] < 10 && pixel[3] == 255)
            );
            assert_ne!(red, blue);
        })
        .join()
        .unwrap();
    }
}
