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
    /// 越界返回 None；revision 耗尽先记录 ERROR 再终止，避免版本回绕。
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        let entry = self.items.get_mut(index)?;
        let Some(revision) = entry.revision.checked_add(1) else {
            widgetry_error!(index, id = ?entry.id, revision = entry.revision, "ListModel 内容 revision 已耗尽");
            panic!("WidgetryListModel revision exhausted");
        };
        entry.revision = revision;
        Some(&mut entry.value)
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

    /// 在末尾添加新 entry，返回从未使用过的 model-local id。
    pub fn push(&mut self, value: T) -> WidgetryListItemId {
        let entry = self.new_entry(value);
        let id = entry.id;
        self.items.push(entry);
        id
    }

    /// 在 index 前插入；index == len 允许追加，越界返回原 value 且不分配 id。
    pub fn insert(&mut self, index: usize, value: T) -> Result<WidgetryListItemId, T> {
        if index > self.items.len() {
            return Err(value);
        }
        let entry = self.new_entry(value);
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
    fn new_entry(&mut self, value: T) -> ListEntry<T> {
        let id = WidgetryListItemId(self.next_id);
        let Some(next_id) = self.next_id.checked_add(1) else {
            widgetry_error!(next_id = self.next_id, "ListModel item id 已耗尽");
            panic!("WidgetryListModel item id exhausted");
        };
        self.next_id = next_id;
        ListEntry {
            id,
            revision: 0,
            disabled: false,
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// push 与 insert 分配独立 id，删除再插入同值也不能复用旧 identity。
    #[test]
    fn ids_are_unique_and_not_reused() {
        let mut model = WidgetryListModel::default();
        let first = model.push("first");
        let second = model.push("second");
        let inserted = model.insert(1, "inserted").unwrap();
        assert_ne!(first, second);
        assert_ne!(inserted, first);
        assert_ne!(inserted, second);
        assert_eq!(model.remove(0), Some("first"));
        assert_eq!(model.index_of(first), None);
        assert_eq!(model.get_by_id(first), None);
        assert_ne!(model.push("first"), first);
    }

    /// mutable access 只推进目标内容版本，disabled metadata 的改变不推进 revision。
    #[test]
    fn revisions_are_local_and_independent_from_disabled() {
        let mut model = WidgetryListModel::default();
        model.push(10);
        model.push(20);
        assert_eq!(model.revision(0), Some(0));
        *model.get_mut(1).unwrap() = 21;
        assert_eq!(model.get(1), Some(&21));
        assert_eq!(model.revision(0), Some(0));
        assert_eq!(model.revision(1), Some(1));
        assert!(model.set_disabled(1, true));
        assert_eq!(model.is_disabled(1), Some(true));
        assert_eq!(model.revision(1), Some(1));
        assert_eq!(model.is_disabled(0), Some(false));
        assert_eq!(model.get_mut(2), None);
        assert!(!model.set_disabled(2, true));
        assert_eq!(model.revision(2), None);
        assert_eq!(model.is_disabled(2), None);
        model.get_mut(1).unwrap();
        assert_eq!(model.revision(1), Some(2));
    }

    /// move 的 to 是最终 index，前后移动保留 id、revision 和 disabled；越界不改变顺序。
    #[test]
    fn move_preserves_entries_and_uses_final_index() {
        let mut model = WidgetryListModel::default();
        let a = model.push('a');
        let b = model.push('b');
        let c = model.push('c');
        model.get_mut(0).unwrap();
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
        assert_eq!(model.insert(4, 'd'), Err('d'));
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
        let first = model.push(1);
        assert!(model.move_item(0, 0));
        let second = model.push(2);
        model.clear();
        assert!(model.is_empty());
        assert_eq!(model.len(), 0);
        assert_eq!(model.index_of(first), None);
        assert_eq!(model.get_by_id(second), None);
        let next = model.push(1);
        assert_ne!(next, first);
        assert_ne!(next, second);
        assert_eq!(model.id(0), Some(next));
        assert_eq!(model.get_by_id(next), Some(&1));
    }
}
