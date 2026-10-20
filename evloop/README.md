# evloop - Yet Another Window Abstraction Layer for Rust

```toml
[dependencies]
evloop = "<UNSPECIFIED>"
```

## Why not use winit?

While `winit` provides a well-established cross-platform windowing abstraction, `evloop` takes a different approach to platform abstraction.

### Platform-specific APIs without unnecessary trait abstractions

We believe that APIs should share signatures and semantics where practical, without requiring platform-specific implementations to conform to a common set of traits.

For example, an Android `Window` struct cannot be used as a Win32 `Window`. These types represent fundamentally different platform resources, and there is no inherent need for them to share a trait merely to provide a consistent API.

Instead, `evloop` aims to expose platform-specific types with consistent APIs, allowing each backend to retain the flexibility to express platform-specific behavior without being constrained by a shared trait hierarchy.

Libraries that need to support multiple platforms can select the appropriate implementation through conditional compilation (cfg). At the application level, a facade can re-export the selected backend's types behind a consistent public API.

### Share data types, not necessarily behavior

We distinguish between shared data types and shared behavioral interfaces.

Types such as positions, sizes, and other geometry primitives have platform-independent meanings and are natural candidates for a common crate. Sharing these types enables interoperability and avoids unnecessary duplication.

Behavioral interfaces such as `Window` and `ApplicationHandler`, however, do not inherently need to be shared through traits. Their implementations are platform-specific, and the primary goal is to provide a consistent API rather than to make distinct platform resources interchangeable.

Traits remain useful when genuine polymorphism is needed, such as when a library must operate on multiple implementations within the same compilation unit. We simply do not consider them a prerequisite for cross-platform API consistency.

`evloop` follows this distinction: share abstractions where they provide tangible value, and keep platform-specific APIs independent where they do not need to be unified.

## Usage

`evloop` is window creation and management library. It can create windows and lets you handle events (for example: the window being resized, a key being pressed, a mouse movement, etc.) produced by the window, activity, process.

`evloop` is designed to provide a low-level interface for handling platform events and lifecycle changes, while preserving access to platform-specific functionality. It provides the foundation for building applications that can respond naturally to the behavior and capabilities of each platform.

