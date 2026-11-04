use std::process::ExitCode;

fn main() -> ExitCode {
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    tracing::subscriber::with_default(subscriber, || {
        match bevy_widgetry_app_icon_build::set_exe_icon("src/assets/icons/widget_gallery_exe.ico")
        {
            Ok(()) => ExitCode::SUCCESS,
            Err(failure) => {
                eprintln!(
                    "图标构建因未处理错误终止 phase=unhandled error_id={} origin={} context={:?} backtrace={}",
                    failure.origin().id(),
                    failure.origin().location(),
                    failure.contexts(),
                    failure.origin().backtrace()
                );
                ExitCode::FAILURE
            }
        }
    })
}
