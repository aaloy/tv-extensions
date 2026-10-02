# Host-driven apps

Turbo Vision was written for a program that owns its terminal. `Application::run` sits in a loop, asks the backend for the next event, waits up to a few milliseconds for one, and draws whenever something changed. That shape does not fit an embedder. A plank frame, a WASM guest or a test harness has its own loop and its own screen, and it calls into the application when it has something to hand over: a key, a resize, or just the next frame. The `host` module turns turbo-vision inside out so it can be driven that way.

## The pieces

`HostBackend` is a `Backend` with no terminal behind it. Its input is a queue, its size is whatever the host last said, and its output goes nowhere, because the host reads the finished cells straight out of `Terminal::buffer`. `HostInput` is the host's end of that queue. It is cheap to clone, so it can live wherever the host receives its input, and it offers `push` for an event and `set_size` for a new screen size. `host::app(w, h)` builds the three together and returns the application with its input handle.

`pump(&mut app, &mut handler)` is one frame. It first checks whether the host changed the size, and if so it runs the same re-layout a terminal resize gets. Then it takes every queued event in order and gives each one to `Application::step`, which draws any pending redraw and then does one pass of the event loop, the pass `run` does for each event it reads. When the queue is empty it steps once more with no event, which is where idle work and `AppHandler::idle` run, and finally it draws. It returns whether the application is still running, and turns `false` once something quits it with `CM_QUIT` or Alt+X.

Nothing in this path waits. `HostBackend::poll_event` returns the next queued event or nothing, immediately, so a frame costs only the work the events cause.

## Why modal calls are refused

Much of Turbo Vision's API is modal. `exec_view`, `execute_modal`, a message box, the history list under an input line and a dropdown's list all run a nested event loop and return only once the user closes them. Under a host that is a trap: the nested loop would poll a queue nobody can fill while the application holds the host's thread, so the frame would never return.

A `HostBackend` answers `true` to `Backend::is_host_driven`, and turbo-vision reads that once, when the application is built. From then on those calls return `CM_CANCEL` straight away instead of looping, and the history and dropdown popups simply do not open. The tests pin this down by pushing `CM_SHOW_HISTORY` and `CM_SHOW_DROPDOWN` and checking that a pump comes back promptly.

The consequence for an application is that dialogs have to be modeless. Add the dialog to the desktop, keep a note of what it is for, and act on its commands in the handler as they arrive. plank's csvedit guest keeps an `overlay` field for exactly this: its open, save and confirm dialogs are ordinary desktop children, and the handler closes them and applies the answer when their buttons send a command.

## The plank frame example

A plank frame component receives a key at a time and is asked to paint at a given size. csvedit wraps its application in a small session that maps each of those onto a pump:

```rust
use tv_extensions::host::{self, HostInput};
use turbo_vision::app::Application;
use turbo_vision::core::event::Event;

pub struct Session {
    app: Application,
    input: HostInput,
    state: State, // the AppHandler: document, handles, open overlay
}

impl Session {
    pub fn key(&mut self, ev: Event) -> bool {
        self.input.push(ev);
        host::pump(&mut self.app, &mut self.state)
    }

    pub fn step(&mut self, w: u16, h: u16) {
        self.input.set_size(w, h);
        host::pump(&mut self.app, &mut self.state);
    }

    pub fn cells(&self) -> &[Vec<turbo_vision::core::draw::Cell>] {
        self.app.terminal.buffer()
    }
}
```

Keys go in through the queue and come out as a redrawn buffer in the same call, and a resize is just a new size followed by a pump. When `key` returns `false` the application has quit and the frame can close.

One detail of layout is worth knowing. Without a menu bar and a status line, turbo-vision's desktop keeps a spare row at the top and bottom, as Borland's does. An embedder that wants the whole screen for its windows either sets a menu bar and status line, as csvedit does, or sets the desktop's bounds to the full screen.

## The cursor

A host-driven application has no terminal cursor of its own to fall back on, so `HostInput::cursor()` reports where the application last put one: the screen cell `(x, y)` a focused control showed its text cursor at after the last `pump`, or `None` while nothing shows one. A host that can place a real cursor moves it there each frame; one that can't can instead redraw that cell in reverse video.
