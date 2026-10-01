use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;

/// 仅在所属 model 生命周期内唯一；完整 identity 还包含 source entity。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WidgetryListItemId(u64);

/// 将稳定 identity、内容版本与持久 disabled metadata 绑定到 value。
struct ListEntry<T> {
    /// 移动时保持、删除后作废的 model-local identity。
    id: WidgetryListItemId,
    /// 只在取得 value 的 mutable access 时推进。
    revision: u64,
    /// 与内容 revision 正交，不受 ListView root disabled 影响。
    disabled: bool,
    /// 唯一业务内容，不允许绕过 model API 取得 mutable access。
    value: T,
}

/// ListView data 的唯一 source of truth；多个 view 可共享同一个 model。
/// 通过 method 增删改移；不支持整块替换 component，否则会破坏持续 identity。
#[derive(Component)]
pub struct WidgetryListModel<T: Send + Sync + 'static> {
    /// 有序 entry；不对调用方暴露裸 Vec。
    items: Vec<ListEntry<T>>,
    /// 单调分配且不因 clear 或 remove 回退。
    next_id: u64,
}

impl<T: Send + Sync + 'static> Default for WidgetryListModel<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            next_id: 0,
        }
    }
}

impl<T: Send + Sync + 'static> WidgetryListModel<T> {
    /// 当前 entry 数量，不包含已删除的 item。
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// model 是否没有任何 entry。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 读取当前 index 对应的稳定 id；越界返回 None。
    pub fn id(&self, index: usize) -> Option<WidgetryListItemId> {
        self.items.get(index).map(|entry| entry.id)
    }

    /// 清空所有 entry，使旧 id 全部失效，但不重置 id 分配 counter。
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// 移到最终 index to，保留 id、revision 和 disabled；不使用插入前 index 语义。
    /// from/to 均必须小于 len，同 index 成功但不改变内容，越界返回 false。
    pub fn move_item(&mut self, from: usize, to: usize) -> bool {
        if from >= self.items.len() || to >= self.items.len() {
            return false;
        }
        if from != to {
            let entry = self.items.remove(from);
            self.items.insert(to, entry);
        }
        true
    }

    /// 按当前 index 读取业务内容；越界返回 None。
    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index).map(|entry| &entry.value)
    }

    /// 返回 mutable value 前推进目标 revision，即使调用方最终没有修改内容。
    /// 越界返回 Ok(None)；revision 耗尽记录 ERROR 并返回 BevyError，保持原 state，避免版本回绕。
    pub fn get_mut(&mut self, index: usize) -> Result<Option<&mut T>, BevyError> {
        let Some(entry) = self.items.get_mut(index) else {
            return Ok(None);
        };
        let Some(revision) = entry.revision.checked_add(1) else {
            widgetry_error!(index, id = ?entry.id, revision = entry.revision, "ListModel 内容 revision 已耗尽");
            return Err(BevyError::error("WidgetryListModel revision exhausted"));
        };
        entry.revision = revision;
        Ok(Some(&mut entry.value))
    }

    /// 读取内容版本，disabled 和移动不改变此值；越界返回 None。
    pub fn revision(&self, index: usize) -> Option<u64> {
        self.items.get(index).map(|entry| entry.revision)
    }

    /// 更新持久 disabled metadata，不推进内容 revision；越界返回 false。
    pub fn set_disabled(&mut self, index: usize, disabled: bool) -> bool {
        let Some(entry) = self.items.get_mut(index) else {
            return false;
        };
        entry.disabled = disabled;
        true
    }

    /// 读取持久 disabled metadata；越界返回 None。
    pub fn is_disabled(&self, index: usize) -> Option<bool> {
        self.items.get(index).map(|entry| entry.disabled)
    }

    /// 在末尾添加新 entry，返回从未使用过的 model-local id；id 耗尽记录 ERROR 并返回 BevyError。
    pub fn push(&mut self, value: T) -> Result<WidgetryListItemId, BevyError> {
        let entry = self.new_entry(value)?;
        let id = entry.id;
        self.items.push(entry);
        Ok(id)
    }

    /// 在 index 前插入；index == len 允许追加，越界或 id 耗尽记录 ERROR 并返回 BevyError，不改变 model。
    pub fn insert(&mut self, index: usize, value: T) -> Result<WidgetryListItemId, BevyError> {
        if index > self.items.len() {
            widgetry_error!(index, len = self.items.len(), "ListModel 插入 index 越界");
            return Err(BevyError::error(
                "WidgetryListModel insertion index out of bounds",
            ));
        }
        let entry = self.new_entry(value)?;
        let id = entry.id;
        self.items.insert(index, entry);
        Ok(id)
    }

    /// 删除 entry 并使其 id 永久失效；越界返回 None。
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.items.len() {
            return None;
        }
        Some(self.items.remove(index).value)
    }

    /// 通过稳定 id 查找当前 index；已删除的 id 返回 None。
    /// 不同 model 的 id 可能相同，调用方必须同时保存所属 source entity。
    pub fn index_of(&self, id: WidgetryListItemId) -> Option<usize> {
        self.items.iter().position(|entry| entry.id == id)
    }

    /// 按稳定 id 读取 value，不暴露 private entry。
    pub fn get_by_id(&self, id: WidgetryListItemId) -> Option<&T> {
        self.index_of(id).map(|index| &self.items[index].value)
    }

    /// 集中分配 identity，拒绝 counter 溢出以保证永不复用。
    fn new_entry(&mut self, value: T) -> Result<ListEntry<T>, BevyError> {
        let id = WidgetryListItemId(self.next_id);
        let Some(next_id) = self.next_id.checked_add(1) else {
            widgetry_error!(next_id = self.next_id, "ListModel item id 已耗尽");
            return Err(BevyError::error("WidgetryListModel item id exhausted"));
        };
        self.next_id = next_id;
        Ok(ListEntry {
            id,
            revision: 0,
            disabled: false,
            value,
        })
    }
}

// 测试 module 中的断言用于验证 contract，生产代码仍禁止。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::log::tracing::Level;
    use bevy_widgetry_test_utils::LogCapture;
    use proptest::prelude::*;
    use std::collections::HashSet;

    /// 测试账本独立保存顺序及全部 metadata，不从 model 反向生成 expected。
    #[derive(Clone, Debug)]
    struct ExpectedEntry {
        id: WidgetryListItemId,
        value: i16,
        revision: u64,
        disabled: bool,
    }

    /// 有界业务操作；失败输入直接显示操作名与位置选择，便于重现。
    #[derive(Clone, Debug)]
    enum Op {
        Push(i16),
        Insert(u8, u8, i16),
        Remove(u8, u8),
        Move(u8, u8, u8, u8),
        Clear,
        Set(u8, u8, i16),
        Touch(u8, u8),
        Disable(u8, u8, bool),
    }

    /// Push/Insert 与删除混合，合法位置相对于执行时长度解释，序列最多 128 步。
    fn operation() -> impl Strategy<Value = Op> {
        prop_oneof![
            3 => (-100i16..100).prop_map(Op::Push),
            3 => (any::<u8>(), 0u8..4, -100i16..100).prop_map(|(raw, mode, value)| Op::Insert(raw, mode, value)),
            2 => (any::<u8>(), 0u8..4).prop_map(|(raw, mode)| Op::Remove(raw, mode)),
            2 => (any::<u8>(), 0u8..4, any::<u8>(), 0u8..4).prop_map(|(raw, mode, to, to_mode)| Op::Move(raw, mode, to, to_mode)),
            1 => Just(Op::Clear),
            2 => (any::<u8>(), 0u8..4, -100i16..100).prop_map(|(raw, mode, value)| Op::Set(raw, mode, value)),
            2 => (any::<u8>(), 0u8..4).prop_map(|(raw, mode)| Op::Touch(raw, mode)),
            2 => (any::<u8>(), 0u8..4, any::<bool>()).prop_map(|(raw, mode, disabled)| Op::Disable(raw, mode, disabled)),
        ]
    }

    /// 根据当前长度选择合法、末尾边界、越界和首项，避免随机输入退化成无效操作。
    fn index(raw: u8, mode: u8, len: usize) -> usize {
        match mode {
            0 => usize::from(raw) % len.max(1),
            1 => len,
            2 => len + 1,
            _ => 0,
        }
    }

    /// 当前顺序、查询、版本与失效历史必须在每个操作后同时成立。
    fn assert_ledger(
        model: &WidgetryListModel<i16>,
        expected: &[ExpectedEntry],
        retired: &HashSet<WidgetryListItemId>,
    ) {
        assert_eq!(model.len(), expected.len());
        assert_eq!(model.is_empty(), expected.is_empty());
        let mut current = HashSet::new();
        for (position, entry) in expected.iter().enumerate() {
            assert!(current.insert(entry.id));
            assert!(!retired.contains(&entry.id));
            assert_eq!(model.id(position), Some(entry.id));
            assert_eq!(model.get(position), Some(&entry.value));
            assert_eq!(model.revision(position), Some(entry.revision));
            assert_eq!(model.is_disabled(position), Some(entry.disabled));
            assert_eq!(model.index_of(entry.id), Some(position));
            assert_eq!(model.get_by_id(entry.id), Some(&entry.value));
        }
        assert_eq!(model.id(expected.len()), None);
        assert_eq!(model.get(expected.len()), None);
        assert_eq!(model.revision(expected.len()), None);
        assert_eq!(model.is_disabled(expected.len()), None);
        for id in retired {
            assert_eq!(model.index_of(*id), None);
            assert_eq!(model.get_by_id(*id), None);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// 每步精确核对返回值与独立账本；默认 failure persistence 保留可缩减重现序列。
        #[test]
        fn operation_sequences_preserve_order_identity_and_metadata(
            operations in prop::collection::vec(operation(), 1..129)
        ) {
            let mut model = WidgetryListModel::default();
            let mut expected: Vec<ExpectedEntry> = Vec::new();
            let mut allocated = HashSet::new();
            let mut retired = HashSet::new();
            for (step, op) in operations.iter().enumerate() {
                match *op {
                    Op::Push(value) | Op::Insert(_, _, value) => {
                        let at = match *op { Op::Insert(raw, mode, _) => index(raw, mode, expected.len()), _ => expected.len() };
                        let result = match *op { Op::Push(_) => Ok(model.push(value).unwrap()), _ => model.insert(at, value) };
                        if at <= expected.len() {
                            let id = result.unwrap();
                            assert_eq!(id, WidgetryListItemId(allocated.len() as u64), "step {step}: {op:?}");
                            assert!(allocated.insert(id), "step {step}: identity reused");
                            expected.insert(at, ExpectedEntry { id, value, revision: 0, disabled: false });
                        } else {
                            assert!(result.is_err(), "step {step}: {op:?}");
                        }
                    }
                    Op::Remove(raw, mode) => {
                        let position = index(raw, mode, expected.len());
                        let removed = if position < expected.len() { Some(expected.remove(position)) } else { None };
                        assert_eq!(model.remove(position), removed.as_ref().map(|entry| entry.value), "step {step}: {op:?}");
                        if let Some(entry) = removed { retired.insert(entry.id); }
                    }
                    Op::Move(raw, mode, raw_to, mode_to) => {
                        let position = index(raw, mode, expected.len());
                        let to = index(raw_to, mode_to, expected.len());
                        let valid = position < expected.len() && to < expected.len();
                        assert_eq!(model.move_item(position, to), valid, "step {step}: {op:?}");
                        if valid {
                            // 用最终位置映射排序，独立表达其他 entry 的相对顺序。
                            let mut mapped: Vec<_> = expected.drain(..).enumerate().map(|(old, entry)| {
                                let new = if old == position { to }
                                    else if position < to && (position + 1..=to).contains(&old) { old - 1 }
                                    else if to < position && (to..position).contains(&old) { old + 1 }
                                    else { old };
                                (new, entry)
                            }).collect();
                            mapped.sort_by_key(|(new, _)| *new);
                            expected = mapped.into_iter().map(|(_, entry)| entry).collect();
                        }
                    }
                    Op::Clear => {
                        retired.extend(expected.drain(..).map(|entry| entry.id));
                        model.clear();
                    }
                    Op::Set(raw, mode, _) | Op::Touch(raw, mode) => {
                        let position = index(raw, mode, expected.len());
                        let actual = model.get_mut(position).unwrap();
                        assert_eq!(actual.is_some(), position < expected.len(), "step {step}: {op:?}");
                        if let Some(actual) = actual {
                            expected[position].revision += 1;
                            if let Op::Set(_, _, value) = *op { *actual = value; expected[position].value = value; }
                        }
                    }
                    Op::Disable(raw, mode, disabled) => {
                        let position = index(raw, mode, expected.len());
                        assert_eq!(model.set_disabled(position, disabled), position < expected.len(), "step {step}: {op:?}");
                        if position < expected.len() { expected[position].disabled = disabled; }
                    }
                }
                assert_ledger(&model, &expected, &retired);
            }
        }
    }

    /// id 耗尽返回 Error severity 的错误并记录 ERROR，不能回绕或改变已有 entry。
    #[test]
    fn exhausted_identity_returns_error_without_mutation() {
        let mut model = WidgetryListModel::default();
        let id = model.push(7).unwrap();
        model.next_id = u64::MAX;
        let capture = LogCapture::default();
        let error = capture.run(|| model.push(8)).unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        let records = capture.records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].level, Level::ERROR);
        assert!(records[0].fields["message"].contains("item id 已耗尽"));
        assert_eq!(model.next_id, u64::MAX);
        assert_eq!(model.len(), 1);
        assert_eq!(model.id(0), Some(id));
        assert_eq!(model.get(0), Some(&7));
    }

    /// revision 耗尽拒绝 mutable access，原 value、disabled 与 revision 保持不变。
    #[test]
    fn exhausted_revision_returns_error_without_mutation() {
        let mut model = WidgetryListModel::default();
        let id = model.push(7).unwrap();
        model.items[0].revision = u64::MAX;
        model.set_disabled(0, true);
        let capture = LogCapture::default();
        let error = capture.run(|| model.get_mut(0)).unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        let records = capture.records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].level, Level::ERROR);
        assert!(records[0].fields["message"].contains("revision 已耗尽"));
        assert_eq!(model.id(0), Some(id));
        assert_eq!(model.get(0), Some(&7));
        assert_eq!(model.revision(0), Some(u64::MAX));
        assert_eq!(model.is_disabled(0), Some(true));
    }

    /// push 与 insert 分配独立 id，删除再插入同值也不能复用旧 identity。
    #[test]
    fn ids_are_unique_and_not_reused() {
        let mut model = WidgetryListModel::default();
        let first = model.push("first").unwrap();
        let second = model.push("second").unwrap();
        let inserted = model.insert(1, "inserted").unwrap();
        assert_ne!(first, second);
        assert_ne!(inserted, first);
        assert_ne!(inserted, second);
        assert_eq!(model.remove(0), Some("first"));
        assert_eq!(model.index_of(first), None);
        assert_eq!(model.get_by_id(first), None);
        assert_ne!(model.push("first").unwrap(), first);
    }

    /// mutable access 只推进目标内容版本，disabled metadata 的改变不推进 revision。
    #[test]
    fn revisions_are_local_and_independent_from_disabled() {
        let mut model = WidgetryListModel::default();
        model.push(10).unwrap();
        model.push(20).unwrap();
        assert_eq!(model.revision(0), Some(0));
        *model.get_mut(1).unwrap().unwrap() = 21;
        assert_eq!(model.get(1), Some(&21));
        assert_eq!(model.revision(0), Some(0));
        assert_eq!(model.revision(1), Some(1));
        assert!(model.set_disabled(1, true));
        assert_eq!(model.is_disabled(1), Some(true));
        assert_eq!(model.revision(1), Some(1));
        assert_eq!(model.is_disabled(0), Some(false));
        assert_eq!(model.get_mut(2).unwrap(), None);
        assert!(!model.set_disabled(2, true));
        assert_eq!(model.revision(2), None);
        assert_eq!(model.is_disabled(2), None);
        model.get_mut(1).unwrap().unwrap();
        assert_eq!(model.revision(1), Some(2));
    }

    /// move 的 to 是最终 index，前后移动保留 id、revision 和 disabled；越界不改变顺序。
    #[test]
    fn move_preserves_entries_and_uses_final_index() {
        let mut model = WidgetryListModel::default();
        let a = model.push('a').unwrap();
        let b = model.push('b').unwrap();
        let c = model.push('c').unwrap();
        model.get_mut(0).unwrap().unwrap();
        model.set_disabled(0, true);
        assert!(model.move_item(0, 2));
        assert_eq!(
            (model.id(0), model.id(1), model.id(2)),
            (Some(b), Some(c), Some(a))
        );
        assert_eq!(model.revision(2), Some(1));
        assert_eq!(model.is_disabled(2), Some(true));
        assert!(model.move_item(2, 0));
        assert!(model.move_item(1, 1));
        assert_eq!(model.get(0), Some(&'a'));
        assert_eq!(model.id(1), Some(b));
        assert!(!model.move_item(0, 3));
        assert!(!model.move_item(3, 0));
        assert_eq!(model.id(0), Some(a));
        assert_eq!(model.index_of(a), Some(0));
        assert_eq!(model.index_of(c), Some(2));
        assert_eq!(model.id(3), None);
        assert_eq!(model.remove(3), None);
        assert!(model.insert(4, 'd').is_err());
        assert_eq!(model.len(), 3);
        assert!(!model.is_empty());
        let tail = model.insert(3, 'd').unwrap();
        assert_eq!(model.index_of(tail), Some(3));
    }

    /// clear 使所有旧 id 失效，后续追加不复用；空 model 和单 entry move 保持边界约定。
    #[test]
    fn clear_keeps_identity_counter_and_handles_empty_moves() {
        let mut model = WidgetryListModel::default();
        assert!(model.is_empty());
        assert!(!model.move_item(0, 0));
        let first = model.push(1).unwrap();
        assert!(model.move_item(0, 0));
        let second = model.push(2).unwrap();
        model.clear();
        assert!(model.is_empty());
        assert_eq!(model.len(), 0);
        assert_eq!(model.index_of(first), None);
        assert_eq!(model.get_by_id(second), None);
        let next = model.push(1).unwrap();
        assert_ne!(next, first);
        assert_ne!(next, second);
        assert_eq!(model.id(0), Some(next));
        assert_eq!(model.get_by_id(next), Some(&1));
    }
}
