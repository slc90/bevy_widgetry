# Phase 2.2 Renderer Registry

## 目标

支持 ECS 异构节点渲染。

## 设计

使用泛型注册：

```rust
register_renderer::<T>(renderer)
```

通过 Component 类型匹配。

不要使用：
- priority matcher
- fallback renderer

## 渲染流程

Entity

-> 查找 Component 类型

-> Renderer<T>

-> 生成节点内容

## 缺少 Renderer

当前阶段：

panic。

以后统一 error system 时再调整。

## 测试

Unit：
- renderer 注册
- type lookup

Integration：
- FolderNode 使用 FolderRenderer
- FileNode 使用 FileRenderer
