# View events

A self-contained example of a custom Rerun view sending a typed event to its embedding application.

From the repository root:

```sh
cargo run -p view_events
```

The viewer opens a **View events** pane and a text document showing the host's received-event count.
Click **Send event** to increment the count and display the source view ID.
No server, dataset, or second process is needed.

- [src/event_view.rs](src/event_view.rs) sends `ViewEventKind::Empty` through `ctx.command_sender().send_view_event(...)` only when the button is clicked.
- [src/main.rs](src/main.rs) handles `StartupOptions::on_view_event`, counts the events, and logs the result back through the normal SDK data stream.

`ViewClassUiOutput` still carries only diagnostics.
The event is an owned message containing its recording ID, view ID, and typed kind; the viewer invokes the host after the view UI returns.
`Empty` is the current payload-free variant, not a dynamic JSON payload.
The host callback runs on the UI thread, so slow work should be sent to a worker.
`StartupOptions::on_event` remains separate for viewer-state changes such as playback and selection.
