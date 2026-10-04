use std::ffi::OsStr;

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WidgetryFileDialogFilterId(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WidgetryFileDialogFilter {
    pub id: WidgetryFileDialogFilterId,
    pub label: String,
    pub suffixes: Vec<String>,
    pub default_extension: Option<String>,
}

impl Default for WidgetryFileDialogFilter {
    fn default() -> Self {
        Self {
            id: WidgetryFileDialogFilterId::default(),
            label: "All Files".into(),
            suffixes: Vec::new(),
            default_extension: None,
        }
    }
}

impl WidgetryFileDialogFilter {
    pub(crate) fn validate(&self) -> Result<(), bevy::prelude::BevyError> {
        if self.id.0.is_empty() && !self.suffixes.is_empty() {
            return Err(crate::model::contract_error(
                "empty FilterId is reserved for All Files",
            ));
        }
        for suffix in self.suffixes.iter().chain(self.default_extension.iter()) {
            if suffix.is_empty()
                || suffix.starts_with('.')
                || suffix.ends_with('.')
                || suffix
                    .chars()
                    .any(|character| character.is_control() || "/\\:*?\"<>|".contains(character))
            {
                return Err(crate::model::contract_error("invalid filter suffix"));
            }
        }
        if let Some(extension) = &self.default_extension
            && !self.suffixes.is_empty()
            && !self
                .suffixes
                .iter()
                .any(|suffix| suffix.eq_ignore_ascii_case(extension))
        {
            return Err(crate::model::contract_error(
                "default extension must match filter suffix",
            ));
        }
        Ok(())
    }

    pub fn matches(&self, name: &OsStr) -> bool {
        let name = name.as_encoded_bytes();
        self.suffixes.is_empty() || {
            self.suffixes.iter().any(|suffix| {
                name.len() > suffix.len() + 1
                    && name[name.len() - suffix.len() - 1] == b'.'
                    && name
                        .get(name.len() - suffix.len()..)
                        .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix.as_bytes()))
            })
        }
    }
}

#[cfg(test)]
// 测试允许断言，workspace 的 macro 禁令只约束生产代码。
#[allow(clippy::disallowed_macros)]
mod tests {
    //! filter 区分 All Files、ASCII case、点文件和复合 suffix。
    //! stimuli 为原始 OsStr name，匹配不对路径做大小写归一化。
    use super::*;

    #[test]
    fn suffix_boundaries_and_ascii_case_are_explicit() {
        let mut filter = WidgetryFileDialogFilter {
            id: WidgetryFileDialogFilterId("archive".into()),
            label: "Archive".into(),
            suffixes: vec!["tar.gz".into()],
            default_extension: None,
        };
        assert!(filter.matches(OsStr::new("archive.TAR.GZ")));
        assert!(!filter.matches(OsStr::new("archive.gz")));
        filter.suffixes = vec!["png".into()];
        assert!(filter.matches(OsStr::new("图像.PNG")));
        assert!(!filter.matches(OsStr::new(".png")));
        assert!(!filter.matches(OsStr::new("png")));
        assert!(!filter.matches(OsStr::new("name.png.bak")));
        assert!(WidgetryFileDialogFilter::default().matches(OsStr::new(".hidden")));
    }

    #[test]
    fn invalid_suffix_and_default_extension_are_rejected() {
        let mut filter = WidgetryFileDialogFilter {
            id: WidgetryFileDialogFilterId("text".into()),
            label: "Text".into(),
            suffixes: vec![".txt".into()],
            default_extension: None,
        };
        assert!(filter.validate().is_err());
        filter.suffixes = vec!["txt".into()];
        filter.default_extension = Some("png".into());
        assert!(filter.validate().is_err());
        filter.default_extension = Some("TXT".into());
        assert!(filter.validate().is_ok());
    }
}
