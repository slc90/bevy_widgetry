use bevy::prelude::BevyError;

#[derive(Default)]
pub struct FailureState(Option<String>);

impl FailureState {
    pub fn observe<T>(
        &mut self,
        result: Result<T, BevyError>,
        failed: impl FnOnce(&BevyError),
        recovered: impl FnOnce(),
    ) -> Result<T, BevyError> {
        match &result {
            Err(error) => {
                // BevyError 的 Display 每次附带调用栈。
                // 把整段文本作为 identity 会误报新失败，因此只用错误首行比较异常。
                let display = error.to_string();
                let message = display.lines().next().unwrap_or_default().to_owned();
                if self.0.as_ref() != Some(&message) {
                    failed(error);
                    self.0 = Some(message);
                }
            }
            Ok(_) => {
                if self.0.take().is_some() {
                    recovered();
                }
            }
        }
        result
    }
}
