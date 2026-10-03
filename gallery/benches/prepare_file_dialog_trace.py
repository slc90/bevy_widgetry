import argparse
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib


def replace_once(path, old, new):
    value = path.read_text(encoding="utf-8")
    if value.count(old) != 1:
        raise ValueError(f"rfd source anchor 不唯一或不存在: {path}: {old!r}")
    path.write_text(value.replace(old, new), encoding="utf-8")


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(description="为 Gallery Open File 分段测量准备 Temp 内的 rfd 0.17.2 副本。时间戳先缓存在 thread-local，取消后才输出，不在开窗路径写日志。不修改 registry source 或 Cargo config。")
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    args = parser.parse_args()
    package = tomllib.loads((args.source / "Cargo.toml").read_text(encoding="utf-8"))["package"]
    if package["name"] != "rfd" or package["version"] != "0.17.2":
        parser.error("仅支持已经核对 source anchors 的 rfd 0.17.2")
    source = args.source.resolve()
    destination = args.destination.resolve()
    if destination == source or source in destination.parents:
        parser.error("destination 必须独立于原始 rfd source")
    temporary = Path(tempfile.gettempdir()).resolve()
    if temporary not in destination.parents:
        parser.error("诊断 rfd source 必须位于 Temp 下，不能替换生产依赖 source")
    shutil.copytree(source, destination)
    backend = destination / "src/backend/win_cid"
    utils = backend / "utils.rs"
    with utils.open("a", encoding="utf-8") as output:
        output.write('''
std::thread_local! {
    static PHASES: std::cell::RefCell<Vec<(&'static str, u128)>> = const {
        std::cell::RefCell::new(Vec::new())
    };
}

pub(crate) fn trace_phase(point: &'static str) {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(now) => PHASES.with(|phases| phases.borrow_mut().push((point, now.as_nanos()))),
        Err(error) => eprintln!("@@file_dialog_phase_error {error}"),
    }
}

pub(crate) fn flush_phases() {
    PHASES.with(|phases| {
        for (point, ns) in phases.borrow_mut().drain(..) {
            eprintln!("@@file_dialog_phase {point} {ns}");
        }
    });
}
''')
    replace_once(utils, "    let res =", '    trace_phase("com_begin");\n    let res =')
    replace_once(utils, "    if res < 0 {", '    trace_phase("com_end");\n    if res < 0 {')
    file_dialog = backend / "file_dialog.rs"
    replace_once(file_dialog,
                 "    fn pick_file(self) -> Option<PathBuf> {\n",
                 '    fn pick_file(self) -> Option<PathBuf> {\n        super::utils::trace_phase("rfd_enter");\n')
    replace_once(file_dialog,
                 "                let dialog = IDialog::build_pick_file(&opt)?;\n",
                 '                let dialog = IDialog::build_pick_file(&opt)?;\n                super::utils::trace_phase("show_begin");\n')
    replace_once(file_dialog,
                 "        run(self).ok()\n    }\n\n    fn pick_files",
                 "        let result = run(self).ok();\n        super::utils::flush_phases();\n        result\n    }\n\n    fn pick_files")
    ffi = backend / "file_dialog/dialog_ffi.rs"
    replace_once(ffi,
                 "    fn new_open_dialog(opt: &FileDialog) -> Result<Self> {\n",
                 '    fn new_open_dialog(opt: &FileDialog) -> Result<Self> {\n        super::super::utils::trace_phase("object_begin");\n')
    replace_once(ffi,
                 "        let dialog = unsafe { DialogInner::open()? };\n",
                 '        let dialog = unsafe { DialogInner::open()? };\n        super::super::utils::trace_phase("object_end");\n')
    print(f'临时 Cargo config: [patch.crates-io] rfd = {{ path = "{destination.as_posix()}" }}')
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
