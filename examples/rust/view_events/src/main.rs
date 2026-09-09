//! A custom view sends a typed event; the host logs its response back into the viewer.

use std::cell::Cell;
use std::rc::Rc;

use rerun::external::{eframe, re_log, re_log_channel, re_memory, re_viewer, tokio};

mod event_view;

#[global_allocator]
static GLOBAL: re_memory::AccountingAllocator<mimalloc::MiMalloc> =
    re_memory::AccountingAllocator::new(mimalloc::MiMalloc);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    re_log::setup_logging();
    let main_thread_token = rerun::MainThreadToken::i_promise_i_am_on_the_main_thread();

    re_viewer::run_native_app(
        main_thread_token,
        Box::new(move |cc| {
            Ok(Box::new(create_app(
                main_thread_token,
                cc,
                re_viewer::AppEnvironment::Custom("View events".into()),
            )?))
        }),
        None,
    )?;
    Ok(())
}

fn create_app(
    main_thread_token: rerun::MainThreadToken,
    cc: &eframe::CreationContext<'_>,
    app_env: re_viewer::AppEnvironment,
) -> Result<re_viewer::App, Box<dyn std::error::Error + Send + Sync>> {
    let (sender, receiver) = re_log_channel::log_channel(re_log_channel::LogSource::Sdk);
    let recording = rerun::RecordingStreamBuilder::new("rerun_example_view_events").buffered()?;
    let store_id = recording
        .store_info()
        .ok_or("The example recording is disabled")?
        .store_id
        .clone();

    recording.set_sink(Box::new(rerun::sink::CallbackSink::new(move |messages| {
        for message in messages {
            if sender.send(message.clone().into()).is_err() {
                return;
            }
        }
    })));
    recording.log_static("host", &rerun::TextDocument::new("Received events: 0"))?;
    rerun::blueprint::Blueprint::auto().send(&recording, Default::default())?;

    let received = Cell::new(0_u64);
    let startup_options = re_viewer::StartupOptions {
        persist_state: false,
        on_view_event: Some(Rc::new(move |event| {
            if event.store_id != store_id {
                return;
            }

            if let re_viewer::ViewEventKind::Empty = event.kind {
                received.set(received.get() + 1);
                let text = format!(
                    "Received events: {}\n\nSource view: {}",
                    received.get(),
                    event.view_id
                );
                if let Err(err) = recording.log_static("host", &rerun::TextDocument::new(text)) {
                    re_log::error!("Failed to log the host response: {err}");
                }
            }
        })),
        ..Default::default()
    };

    let mut app = re_viewer::App::new(
        main_thread_token,
        re_viewer::build_info(),
        app_env,
        startup_options,
        cc,
        None,
        re_viewer::AsyncRuntimeHandle::from_current_tokio_runtime_or_wasmbindgen()?,
    );
    app.add_view_class::<event_view::EventView>()?;
    app.add_log_receiver(receiver);
    Ok(app)
}
