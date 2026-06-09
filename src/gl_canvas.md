# gl_canvas.rs

`wxGLCanvas` analog — an OpenGL rendering surface parented on a `Frame` or `Panel`.

## Purpose

`GLCanvas` is a real Win32 child window whose HDC is configured for OpenGL. After `set_current()` you can call OpenGL entry points directly; call `swap_buffers()` once per frame to present.

## Key Types

- **`GLCanvas`** — public struct, wraps `Rc<RefCell<GLCtxInner>>`. Cloneable.
- `GLCtxInner` (private) — `hwnd`, `hdc` (private, `CS_OWNDC`), `hglrc`, `state: GlState`.
- `GlState` (private enum) — `Uninit` (no context), `Detached` (context exists, not current), `Current` (context current on the calling thread).
- **`gl11`** — submodule that re-exports the OpenGL 1.1 entry points that `windows_sys` exposes natively, plus a handful of common constants.

## Constants

- `GLCanvas::DEFAULT_W: u32 = 320`, `DEFAULT_H: u32 = 240`.
- Constants in `gl11` (selection): `GL_COLOR_BUFFER_BIT`, `GL_DEPTH_BUFFER_BIT`, `GL_TRIANGLES`, `GL_QUAD_STRIP`, `GL_QUADS`, `GL_MODELVIEW`, `GL_PROJECTION`, `GL_DEPTH_TEST`, `GL_COLOR`, etc.
- Functions in `gl11` (selection): `glBegin`, `glClear`, `glClearColor`, `glColor3f`, `glEnd`, `glFinish`, `glFlush`, `glLoadIdentity`, `glMatrixMode`, `glOrtho`, `glRotatef`, `glTranslatef`, `glVertex2f`, `glVertex3f`, `glViewport`.

## Constructors

- `GLCanvas::new<W: Window>(parent: &W) -> Self` — 320×240.
- `GLCanvas::with_size<W: Window>(parent: &W, width: u32, height: u32) -> Self`.

## Key Methods

- `set_current(&self) -> bool` — bind this canvas's GL context to the calling thread via `wglMakeCurrent`. Returns `true` on success.
- `make_inactive(&self) -> bool` — release the GL context from the current thread (`wglMakeCurrent(null, null)`).
- `swap_buffers(&self) -> bool` — present the back buffer. Call once per frame.
- `is_current(&self) -> bool` — `true` if a GL context is current on the calling thread.
- `has_context(&self) -> bool` — `true` if a GL context has been created (regardless of whether it is current).
- `as_widget_ref(&self) -> WidgetRef`.
- `hwnd(&self) -> HWND` (Windows) / `0` (stub).

## Quick start

```rust,no_run
use ru_wx::prelude::*;
use ru_wx::gl_canvas::gl11 as gl;

// 1. Create the canvas. Default is 320x240; use `with_size` for any other dims.
let frame = Frame::builder()
    .with_title("gl")
    .with_size(640, 480)
    .build();
let canvas = GLCanvas::new(&frame);    // 320x240
let canvas_big = GLCanvas::with_size(&frame, 640, 480);

// 2. Bind the GL context to the calling thread before calling GL.
assert!(canvas.set_current());

// 3. Issue OpenGL 1.1 entry points via the `gl11` re-export. (The `unsafe`
//    is required by every `gl*` function.)
unsafe {
    gl::glViewport(0, 0, 640, 480);
    gl::glClearColor(0.0, 0.1, 0.2, 1.0);
    gl::glClear(gl::GL_COLOR_BUFFER_BIT);
    gl::glMatrixMode(gl::GL_PROJECTION);
    gl::glLoadIdentity();
    gl::glOrtho(-1.0, 1.0, -1.0, 1.0, -1.0, 1.0);
    gl::glMatrixMode(gl::GL_MODELVIEW);
    gl::glLoadIdentity();

    gl::glBegin(gl::GL_TRIANGLES);
    gl::glColor3f(1.0, 0.0, 0.0); gl::glVertex2f(-0.6, -0.6);
    gl::glColor3f(0.0, 1.0, 0.0); gl::glVertex2f( 0.6, -0.6);
    gl::glColor3f(0.0, 0.0, 1.0); gl::glVertex2f( 0.0,  0.6);
    gl::glEnd();
    gl::glFlush();
}

// 4. Present the back buffer. Call once per frame.
canvas.swap_buffers();

// 5. Release the context if you need to call GL on a different canvas / thread.
canvas.make_inactive();

// 6. For OpenGL 2.0+ (shaders, VBOs, VAOs), load entry points at runtime:
//    let glCreateShader: fn(...) -> ... =
//        std::mem::transmute(wglGetProcAddress(b"glCreateShader\0".as_ptr()));

// 7. Put it all in a paint loop via a Timer:
let canvas_for_tick = canvas.clone();
let timer = Timer::new(&frame);
timer.on_tick(move || {
    if !canvas_for_tick.is_current() {
        canvas_for_tick.set_current();
    }
    unsafe { gl::glClear(gl::GL_COLOR_BUFFER_BIT); }
    canvas_for_tick.swap_buffers();
});
timer.start(std::time::Duration::from_millis(16));
```

The HDC is `CS_OWNDC`, so it's private to the window and re-used across every GL call without `GetDC`/`ReleaseDC`. `Drop` releases the GL context (`wglMakeCurrent(null, null)`) before `wglDeleteContext` to avoid leaking a context on a dead thread, and pairs the `GetDC` from the constructor with `ReleaseDC` on the way out.

## Usage

```rust,no_run
use ru_wx::prelude::*;
use ru_wx::gl_canvas::gl11 as gl;

let app = App::new();
let frame = Frame::builder().with_title("gl").with_size(400, 300).build();
let canvas = GLCanvas::new(&frame);
assert!(canvas.set_current());
unsafe {
    gl::glClearColor(0.0, 0.0, 0.0, 1.0);
    gl::glClear(gl::GL_COLOR_BUFFER_BIT);
}
canvas.swap_buffers();
app.run(frame);
```

## Win32 Notes

- Window class: `RuWxGLCanvas`, registered with `CS_HREDRAW | CS_VREDRAW | CS_OWNDC`. `CS_OWNDC` means the HDC is **private** to the window — we acquire it once at construction and re-use it without `GetDC` / `ReleaseDC` per GL call.
- Pixel format is chosen by `ChoosePixelFormat` on a standard 32-bit RGBA / 24-bit depth / 8-bit stencil `PIXELFORMATDESCRIPTOR` (`PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER`, `PFD_TYPE_RGBA`, `PFD_MAIN_PLANE`).
- `SetPixelFormat` is called exactly once at construction, which is required for a `CS_OWNDC` HDC.
- `wglCreateContext` is used (not `wglCreateContextAttribsARB`); the returned context is OpenGL 1.1 by default.
- The OpenGL 1.1 entry points (`glClear`, `glColor3f`, `glBegin`, `glEnd`, …) are re-exported from `windows_sys::Win32::Graphics::OpenGL` so they can be called as ordinary Rust functions. For OpenGL 2.0+ entry points (shaders, VBOs, VAOs, etc.) use `wglGetProcAddress` at runtime.
- `Drop` releases the GL context (`wglMakeCurrent(null, null)`) before calling `wglDeleteContext`, to avoid leaking a context that is current on a dead thread. It also calls `ReleaseDC(hwnd, hdc)` to balance the `GetDC` in the constructor.
- The WndProc is minimal — only `WM_DESTROY` is handled, and it just releases the `Rc` stored in `GWLP_USERDATA`. All other messages go through `DefWindowProcW`.

## Cross-platform stub

On non-Windows targets the type is constructible (as a width/height record with no real GL context). All GL methods return `false` / `0` and `state()` is always `Uninit`; the type is not actually useful but it keeps code that embeds a `GLCanvas` in a layout compiling.

## Tests

- `default_size_is_positive`.
- `gl11_constants_are_distinct` — `GL_COLOR_BUFFER_BIT != GL_DEPTH_BUFFER_BIT`, `GL_TRIANGLES != GL_QUADS`. Sanity check on the re-exports.
- `new_unattached_state_is_correct` — invariant check on the inner state struct (mostly meaningful on non-Windows).

## See Also

- [`frame.md`](./frame.md) — typical parent of a `GLCanvas`.
- [`panel.md`](./panel.md) — alternate parent.
- [`widget.md`](./widget.md) — `Window` trait, `Widget` trait, `as_widget_ref`.
