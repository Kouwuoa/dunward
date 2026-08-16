# Dunward Developer & Agent Guide (`AGENTS.md`)

This guide provides architectural context, codebase layout, and implementation details for AI agents and developers working on **Dunward**.

---

## 1. Project Overview

**Dunward** is a Rust-based game project built with [Bevy](file:///C:/Users/koada/workspace/dunward/Cargo.toml#L24) for higher-level application structure (ECS, assets, audio, window management) and a custom low-level **Vulkan renderer** implemented in the [`renderer`](file:///C:/Users/koada/workspace/dunward/renderer/Cargo.toml) workspace crate.

### Workspace Structure
* **[`dunward`](file:///C:/Users/koada/workspace/dunward/Cargo.toml)** (Root Crate in [`src/`](file:///C:/Users/koada/workspace/dunward/src)): Application entry point, Bevy plugins, ECS systems, and renderer integration.
* **[`renderer`](file:///C:/Users/koada/workspace/dunward/renderer)**: Standalone custom Vulkan graphics library built using `ash`, `vk-mem`, `glam`, and `bytemuck`.
* **[`shaderpack`](file:///C:/Users/koada/workspace/dunward/renderer/shaderpack)**: Sub-crate responsible for compiling shader source code (GLSL and WGSL) into SPIR-V binaries at build time and embedding them into the executable.

---

## 2. The `renderer` Directory Architecture

The [`renderer`](file:///C:/Users/koada/workspace/dunward/renderer) crate is structured into clean, 2-level domain-driven modules with top-level file documentation across all source files:

### Public API (`renderer/src/`)
* [`lib.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/lib.rs): Exports public domain modules and primary engine types:
  * [`Renderer`](file:///C:/Users/koada/workspace/dunward/renderer/src/renderer.rs#L25) & [`RendererError`](file:///C:/Users/koada/workspace/dunward/renderer/src/renderer.rs#L17)
  * [`Camera`](file:///C:/Users/koada/workspace/dunward/renderer/src/camera.rs#L6)
  * Re-exports [`glam`](file:///C:/Users/koada/workspace/dunward/renderer/Cargo.toml#L12) and [`winit`](file:///C:/Users/koada/workspace/dunward/renderer/Cargo.toml#L20).
* [`renderer.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/renderer.rs): Top-level engine orchestrator managing multi-buffered frames in flight, context lifecycle, and delegating frame workload to domain contexts and subsystems.
* [`camera.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/camera.rs): Right-handed camera matrix calculations, view/projection matrix creation, and orbit/mouse transformation helpers (`mouse_rotate`, `mouse_zoom`).
* [`utils.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/utils.rs): General synchronization and lock-guard utilities.

---

## 3. Domain Modules (`renderer/src/`)

### A. Core (`renderer/src/core/`)
Manages Vulkan instances, physical/logical devices, queues, semaphores, and surfaces:
* [`instance.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/core/instance.rs): Vulkan instance creation, validation layers, debug messenger.
* [`device.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/core/device.rs): Physical device ranking, logical device instantiation, Vulkan 1.3 features (synchronization2, dynamic rendering).
* [`queue.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/core/queue.rs): Hardware queues and family capabilities (`Queue`, `QueueFamily`).
* [`semaphore.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/core/semaphore.rs): `BinarySemaphore`, `TimelineSemaphore`, `WaitSemaphore`, `SignalSemaphore`.
* [`surface.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/core/surface.rs): Window surface abstraction and format queries.
* [`mod.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/core/mod.rs): Aggregates into [`DeviceContext`](file:///C:/Users/koada/workspace/dunward/renderer/src/core/mod.rs#L24).

### B. Display (`renderer/src/display/`)
Manages window display presentation, vsync modes, surface formats, and backbuffer presentation:
* [`swapchain.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/display/swapchain.rs): Low-level swapchain handle, image views, and display surface recreation.
* [`mod.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/display/mod.rs): [`DisplayContext`](file:///C:/Users/koada/workspace/dunward/renderer/src/display/mod.rs#L39), [`PresentTextureBundle`](file:///C:/Users/koada/workspace/dunward/renderer/src/display/mod.rs#L23), [`DisplayPresentError`](file:///C:/Users/koada/workspace/dunward/renderer/src/display/mod.rs#L30).

### C. Commands (`renderer/src/commands/`)
Command pool allocation and typestate command recording:
* [`allocator.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/commands/allocator.rs): [`CommandRecorderAllocator`](file:///C:/Users/koada/workspace/dunward/renderer/src/commands/allocator.rs#L14) per queue family.
* [`transfer.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/commands/transfer.rs): [`TransferCommandRecorder`](file:///C:/Users/koada/workspace/dunward/renderer/src/commands/transfer.rs#L23) for synchronous GPU upload operations.
* [`recorder/`](file:///C:/Users/koada/workspace/dunward/renderer/src/commands/recorder):
  * [`mod.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/commands/recorder/mod.rs): [`CommandRecorder<State>`](file:///C:/Users/koada/workspace/dunward/renderer/src/commands/recorder/mod.rs#L24) typestate machine (`Idle` $\rightarrow$ `Recording` $\rightarrow$ `Executable`).
  * [`barriers.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/commands/recorder/barriers.rs): Layout transitions (`transition_texture`), stage synchronization (`sync_texture`, `sync_texture_same_access`), presentation preparation, and Queue Family Ownership Transfers (QFOT).
  * [`transfers.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/commands/recorder/transfers.rs): Image blitting (`blit_texture_to_texture`), texture resolve, and clear operations (`clear_storage_texture`, `clear_color_texture`, `clear_depth_texture`).
  * [`pipeline.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/commands/recorder/pipeline.rs): Material pipeline binding, push constant updates, compute dispatch, and resource updaters.
* [`mod.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/commands/mod.rs): [`CommandSubsystem`](file:///C:/Users/koada/workspace/dunward/renderer/src/commands/mod.rs#L19).

### D. Resources (`renderer/src/resources/`)
GPU memory, buffers, textures, and descriptor management:
* [`buffer.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/buffer.rs): Low-level Vulkan buffer wrapper.
* [`megabuffer.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/megabuffer.rs): Sub-allocated GPU buffer regions (`Megabuffer`, `AllocatedMegabufferRegion`).
* [`texture.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/texture.rs): `Texture`, `ColorTexture`, `DepthTexture`, `StorageTexture`, `TextureAccess`, `TextureQueueState`.
* [`descriptors/`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/descriptors):
  * [`allocator.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/descriptors/allocator.rs): [`DescriptorAllocator`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/descriptors/allocator.rs#L13) pool allocator with auto-expansion.
  * [`layout_builder.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/descriptors/layout_builder.rs): [`DescriptorSetLayoutBuilder`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/descriptors/layout_builder.rs#L7).
  * [`writer.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/descriptors/writer.rs): [`DescriptorWriter`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/descriptors/writer.rs#L7).
* [`factory.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/factory.rs): [`ResourceFactory`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/factory.rs#L21) for constructing GPU resources.
* [`store.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/store.rs): [`ResourceStore`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/store.rs#L22) owning pooled megabuffers and long-lived textures/samplers.
* [`updater.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/updater.rs): [`ResourceUpdater`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/updater.rs#L10) for batching descriptor set updates during recording.
* [`mod.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/mod.rs): [`ResourceSubsystem`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/mod.rs#L96).

### E. Material (`renderer/src/material/`)
Materials, shaders, pipeline state builders, and GPU data:
* [`shader.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/material/shader.rs): `GraphicsShader` and `ComputeShader` SPIR-V module wrappers.
* [`shader_data.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/material/shader_data.rs): `#[repr(C)]` POD structs (`PerFrameData`, `PerMaterialData`, `PerObjectData`, `PerVertexData`, `PerDrawData`).
* [`graphics.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/material/graphics.rs): `GraphicsMaterialFactoryBuilder`.
* [`compute.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/material/compute.rs): `ComputeMaterialFactoryBuilder`.
* [`mod.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/material/mod.rs): [`Material`](file:///C:/Users/koada/workspace/dunward/renderer/src/material/mod.rs#L22), [`MaterialFactory`](file:///C:/Users/koada/workspace/dunward/renderer/src/material/mod.rs#L62).

### F. Scene (`renderer/src/scene/`)
Geometry, meshes, and models:
* [`vertex.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/scene/vertex.rs): `Vertex` and `VertexInputDescription`.
* [`mesh.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/scene/mesh.rs): `Mesh` with procedural triangle and quad constructors.
* [`model.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/scene/model.rs): `Model` (megabuffer geometry uploads) and `FullscreenQuad` (aspect-ratio corrected presentation).

### G. Frame (`renderer/src/frame/`)
Multi-buffered frame execution and rendering stages:
* [`packet.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/frame/packet.rs): [`FrameRenderPacket`](file:///C:/Users/koada/workspace/dunward/renderer/src/frame/packet.rs#L12) and [`FramePresentPacket`](file:///C:/Users/koada/workspace/dunward/renderer/src/frame/packet.rs#L20).
* [`geometry_stage.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/frame/geometry_stage.rs): Rasterization geometry stage.
* [`lighting_stage.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/frame/lighting_stage.rs): Compute lighting stage.
* [`mod.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/frame/mod.rs): [`FrameContext`](file:///C:/Users/koada/workspace/dunward/renderer/src/frame/mod.rs#L26).

---

## 4. Shader Management & Shaderpack

Shaders in Dunward are stored, compiled, and embedded in a dedicated sub-crate located at [`renderer/shaderpack`](file:///C:/Users/koada/workspace/dunward/renderer/shaderpack).

* **Source Shaders Path**: [`renderer/shaderpack/shaders/`](file:///C:/Users/koada/workspace/dunward/renderer/shaderpack/shaders/)
* **Compiled Output Path**: `renderer/shaderpack/shaders-built/` *(generated during build, excluded from git)*
* **Compilation**: Handled by [`build.rs`](file:///C:/Users/koada/workspace/dunward/renderer/shaderpack/build.rs) (`shaderc` for GLSL, `naga` for WGSL).
* **Binary Embedding**: Embedded via `rust-embed` and accessed with `get_shader_spv(ShaderId)`.

---

## 5. Main Application & Integration (`src/`)

The root crate in [`src/`](file:///C:/Users/koada/workspace/dunward/src) initializes Bevy and integrates the Vulkan renderer:
* [`src/main.rs`](file:///C:/Users/koada/workspace/dunward/src/main.rs): Configures Bevy `App`, window resolution, plugins.
* [`src/render/mod.rs`](file:///C:/Users/koada/workspace/dunward/src/render/mod.rs): `DunwardRenderPlugin` registering `Renderer` as a `NonSend` resource.

---

## 6. Guidelines for AI Agents & Developers

1. **Modifying or Adding Shaders**:
   * Add or edit shader files under [`renderer/shaderpack/shaders/`](file:///C:/Users/koada/workspace/dunward/renderer/shaderpack/shaders/).
   * Update [`ShaderId`](file:///C:/Users/koada/workspace/dunward/renderer/shaderpack/src/lib.rs#L9) enum in [`renderer/shaderpack/src/lib.rs`](file:///C:/Users/koada/workspace/dunward/renderer/shaderpack/src/lib.rs) if adding a new shader.
2. **Vulkan & Thread Safety**:
   * Do not pass `Renderer` across threads. It is registered as a `NonSend` resource in Bevy because `winit` windowing and Vulkan context handles require single-threaded affinity on OS platforms.
3. **Graphics Architecture & Resources**:
   * Dynamic geometry, uniform, and storage buffer allocations should use the megabuffer abstractions ([`megabuffer.rs`](file:///C:/Users/koada/workspace/dunward/renderer/src/resources/megabuffer.rs)).
   * Utilize builders in `resources/descriptors/` and material builders under `material/`.
4. **Code Structure**:
   * Keep modules within the 2-level domain-driven layout (`core/`, `display/`, `commands/`, `resources/`, `material/`, `scene/`, `frame/`).
   * Every file must include a module-level doc comment (`//!`) at the top explaining its purpose.
