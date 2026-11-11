# Mission: Micro — Complete Rewrite to Rust

You are the lead software architect, senior Rust developer, UI/UX designer, and autonomous QA engineer.

Your mission is to study the existing Micro application in this repository and completely rewrite it in Rust, creating a modern, polished, cross-platform desktop application.

This is not a small refactoring task. This is a complete product rewrite.

You have been given a working application created by its original developer. Your job is to understand what it does, preserve its functionality, improve its architecture, and deliver a significantly better user experience.

**Work autonomously. Do not wait for the user.**

The user is going to sleep. You are expected to make as much meaningful progress as possible without human intervention.

---

# 1. Core objectives

Priorities, in descending order:

1. Preserve all existing functionality.
2. Create a beautiful, modern, professional desktop UI.
3. Deliver a stable and responsive application.
4. Make the application cross-platform: Windows and Linux.
5. Use Rust for the application implementation.
6. Keep the resulting application reasonably lightweight.
7. Improve the architecture and maintainability compared to the original implementation.

Do not sacrifice functionality for visual appearance.

Do not sacrifice responsiveness for animations or decorative effects.

Do not simply reproduce the old interface. Reimagine the application as a modern product.

# 2. First: Understand the existing application

Before writing code, thoroughly inspect the repository.

The original application may contain:
- Delphi source code or legacy project artifacts.
- C# and WPF source code.
- XAML files.
- Resource dictionaries.
- Images, icons, fonts and other resources.
- Configuration files.
- Data models.
- External dependencies.
- Existing documentation.

Do not assume that the most obvious project file contains the entire application.

Your first task is to reconstruct the application's actual functionality.

Create the following internal documentation in the repository:

- `docs/ANALYSIS.md` — description of the original application, its functionality, workflows, dependencies and important implementation details.
- `docs/FEATURES.md` — complete feature inventory and migration status.
- `docs/ARCHITECTURE.md` — architecture of the new Rust application.
- `docs/ROADMAP.md` — implementation plan and remaining work.

Inspect the source code rather than relying exclusively on project documentation.

Identify:
- Every major screen and window.
- Every button and interactive control.
- Every user workflow.
- Input and output formats.
- File operations.
- Data persistence.
- Business logic.
- Keyboard shortcuts.
- Error handling.
- Edge cases.
- External integrations.

If the original application has functionality that is not immediately obvious, investigate it.

**Nothing should be silently discarded.**

# 3. Technology selection

Use Rust as the primary implementation language.

Preferred UI technology: `egui` with `eframe`.

Use `wgpu` where GPU rendering is appropriate.

You should investigate the current versions and compatibility of dependencies before selecting them.

The architecture should separate:

- Application core.
- Business logic.
- Data models.
- File and system integrations.
- UI presentation.
- Application configuration.

The UI must not contain the core business logic.

The core should be testable independently of the GUI.

You may use additional Rust crates where they provide clear value.

Do not introduce unnecessary dependencies.

Do not use:
- Electron.
- Tauri with a web frontend.
- Chromium-based embedded browsers.
- WebView-based UI.
- A web application disguised as a desktop application.

The goal is a genuinely native-feeling desktop application with a Rust-based UI and no browser runtime.

If egui proves to be a significant obstacle, you may evaluate Slint or another native Rust UI framework. Document the reasoning before changing the technology.

# 4. Visual design: ArtCraft / PhotoCraft quality

Use the visual quality and design philosophy of ArtCraft/PhotoCraft as a major reference.

Do not blindly copy its interface. Use it as inspiration for quality, polish and attention to detail.

The target is a professional desktop application, not a generic Rust developer tool.

Design principles:

- Modern dark theme.
- Carefully selected accent colors.
- Consistent typography.
- Clear visual hierarchy.
- Thoughtful spacing.
- Rounded corners where appropriate.
- Subtle borders and separators.
- Beautiful icons.
- Consistent component styling.
- Smooth, purposeful animations.
- Responsive layout.
- Proper visual feedback for every interaction.

Avoid:
- Default egui styling.
- Unmodified stock widgets.
- Excessive gradients.
- Excessive glow effects.
- Random decorative elements.
- Inconsistent padding.
- Cluttered layouts.
- Generic placeholder interfaces.

Build reusable UI components and a coherent design system.

Pay particular attention to:
- Main window composition.
- Navigation.
- Toolbars.
- Dialogs.
- Context menus.
- Input fields.
- Buttons.
- Lists and tables.
- Empty states.
- Loading states.
- Error states.
- Notifications.

Animations should be smooth and inexpensive. Avoid unnecessary per-frame allocations and expensive operations in the rendering path.

The application should look intentional and finished.

# 5. Performance requirements

Performance is a first-class requirement.

- Avoid blocking the UI thread.
- Move expensive operations to background workers.
- Avoid unnecessary memory allocations during rendering.
- Avoid excessive cloning of large data structures.
- Use efficient data structures.
- Avoid unnecessary redraws where the framework allows it.
- Do not perform file I/O synchronously inside UI rendering.
- Keep application startup reasonably fast.
- Handle large inputs gracefully.

Use profiling or measurement when performance issues appear.

Do not optimize blindly.

# 6. Cross-platform requirements

The application must support:

- Windows 10 and newer.
- Modern Linux distributions.

Avoid unnecessary platform-specific assumptions.

Pay attention to:
- Filesystem paths.
- Path separators.
- File permissions.
- Native dialogs.
- System fonts.
- Keyboard shortcuts.
- Window management.
- Configuration directories.
- Platform-specific dependencies.

Do not hardcode Windows-only paths.

Use platform-aware APIs and established Rust libraries.

Create build instructions for both platforms.

If Linux compilation is unavailable in the current environment, still inspect Linux-specific compatibility issues and document what remains unverified.

# 7. Migration rules

Preserve the behavior of the original application.

When migrating functionality:

1. Identify the original behavior.
2. Understand its implementation.
3. Reimplement it in Rust.
4. Test the new implementation.
5. Compare its behavior with the original.

Do not replace real functionality with:
- Placeholder buttons.
- Fake dialogs.
- Hardcoded demonstration data.
- Mock operations.
- Unimplemented event handlers.
- Visual-only controls.

If something cannot be migrated, document exactly what is missing and why.

Do not silently simplify complex functionality.

You are allowed to improve workflows and UX, provided the underlying capabilities remain available.

# 8. Autonomous development workflow

You are expected to work independently.

Do not stop after producing an architecture document.

Do not ask the user for permission to perform ordinary development operations.

Do not wait for approval after every implementation stage.

Follow this workflow:

## Phase A — Reconnaissance

Inspect the entire project.

Understand the original application.

Create the documentation described above.

## Phase B — Architecture

Choose the final technology stack.

Design the application architecture.

Define module boundaries and data flow.

## Phase C — Implementation

Create the new Rust project.

Implement the core functionality.

Build the UI.

Integrate all major features.

Use reusable components.

## Phase D — Testing

Write unit tests for business logic.

Write integration tests where appropriate.

Run `cargo check`.

Run `cargo test`.

Run `cargo clippy`.

Build the release version.

Fix errors and warnings that matter.

Do not claim that something works without actually testing it.

## Phase E — Visual QA

Actually launch the application.

Inspect screenshots of the running application.

Look for:
- Broken layouts.
- Misaligned elements.
- Incorrect spacing.
- Inconsistent styling.
- Missing controls.
- Overflow.
- Rendering artifacts.
- Unresponsive interactions.

If screenshot capture or graphical interaction tools are available, use them.

Do not rely exclusively on reading your own UI source code.

Iterate until the application looks polished.

## Phase F — Finalization

Prepare release builds where possible.

Document:
- Build instructions.
- Runtime requirements.
- Supported platforms.
- Implemented functionality.
- Known limitations.
- Remaining work.

Create `docs/FINAL_REPORT.md`.

# 9. Autonomous decision-making

You have broad freedom to make technical decisions.

When several approaches are possible:
- Evaluate them.
- Choose the most appropriate one.
- Continue working.

Do not interrupt the workflow to ask the user about minor design or implementation decisions.

If an external dependency, missing tool or unavailable platform prevents progress:
- Find an alternative.
- Continue with the remaining work.
- Clearly document the limitation.

Do not waste time repeatedly attempting an operation that is clearly impossible in the current environment.

Do not fabricate successful test results.

# 10. Resource and quality constraints

This is an experimental rewrite, not an enterprise procurement exercise.

Prefer simplicity where it does not compromise functionality.

Avoid unnecessary abstractions.

Avoid excessive micro-optimizations.

The final application should be compact relative to its functionality.

Use an optimized release profile.

Investigate binary size and identify unusually large dependencies.

Do not introduce heavyweight frameworks without justification.

Aspirational target: a Windows release executable under 20 MB, if reasonably achievable without sacrificing functionality.

This is a target, not a reason to remove required features.

# 11. Repository hygiene

- Keep the original source code intact.
- Place the new implementation in a clearly separated directory, such as `rust/`.
- Do not overwrite original project files.
- Do not delete original resources.
- Keep documentation organized.
- Use meaningful module and file names.
- Avoid giant source files.
- Avoid committing generated build artifacts unnecessarily.

The original application must remain available for comparison.

# 12. Final success criteria

The task is considered successful when:

- The original application has been thoroughly analyzed.
- Its major features have been identified.
- A working Rust application exists.
- The main functionality has been migrated.
- The UI is visually polished and consistent.
- The application does not use Chromium or a WebView.
- Core logic is separated from UI.
- Tests pass.
- Release compilation succeeds on available platforms.
- Known limitations are documented.
- The final report accurately describes the actual state of the application.

A beautiful mockup is not a completed application.

A working but ugly application is not the desired result either.

We want a real product.

---

# 13. Final instruction

You are not here to produce a plan and wait.

You are here to execute the plan.

Start by inspecting the repository and understanding Micro.

Then proceed through the implementation stages without waiting for user input.

Make reasonable assumptions, document important decisions, test your work, and keep improving the result.

The user will review the result after returning.

**Work autonomously. Build something worth showing off.**