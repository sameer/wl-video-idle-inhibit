use std::ffi::OsStr;

use inotify::{EventMask, Inotify, WatchMask};
use wayland_client::{
    protocol::{
        __interfaces::WL_COMPOSITOR_INTERFACE,
        wl_compositor::{self, WlCompositor},
        wl_registry::{self, WlRegistry},
        wl_surface::{self, WlSurface},
    },
    Connection, Dispatch, QueueHandle,
};
use wayland_protocols::wp::idle_inhibit::zv1::client::{
    __interfaces::ZWP_IDLE_INHIBIT_MANAGER_V1_INTERFACE,
    zwp_idle_inhibit_manager_v1::{self, ZwpIdleInhibitManagerV1},
    zwp_idle_inhibitor_v1::{self, ZwpIdleInhibitorV1},
};

const DEV_PATH: &str = "/dev";
const VIDEO_PREFIX: &str = "video";

fn main() {
    let watch_mask = WatchMask::OPEN | WatchMask::CLOSE;
    let mut inotify = Inotify::init().expect("failed to initialize inotify");
    inotify
        .add_watch(DEV_PATH, watch_mask)
        .expect("couldn't watch for video device events");

    let conn = Connection::connect_to_env().expect("could not connect to Wayland server");
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let display = conn.display();

    let _registry = display.get_registry(&qh, ());

    let mut state = State::default();
    event_queue.blocking_dispatch(&mut state).unwrap();
    let mut idle_inhibitor = None;

    let mut num_active_players = 0usize;
    let mut buf = [0; 1024];
    loop {
        for event in inotify
            .read_events_blocking(&mut buf)
            .expect("error while reading video device events")
        {
            let name = if let Some(name) = event
                .name
                .and_then(OsStr::to_str)
                .filter(|name| name.starts_with(VIDEO_PREFIX))
            {
                name
            } else {
                continue;
            };

            if EventMask::OPEN.contains(event.mask) {
                idle_inhibitor = idle_inhibitor.or_else(|| {
                    let inhibitor = state
                        .idle_inhibit_manager
                        .as_ref()
                        .expect("idle manager should be present")
                        .create_inhibitor(
                            state
                                .surf
                                .as_ref()
                                .expect("wayland surface should be present"),
                            &qh,
                            (),
                        );
                    conn.roundtrip()
                        .expect("failed to request creating idle inhibitor");
                    Some(inhibitor)
                });
                num_active_players += 1;
                println!("Idle inhibited by {}", name);
            } else if (EventMask::CLOSE_WRITE | EventMask::CLOSE_NOWRITE).contains(event.mask) {
                println!("Idle permitted by {}", name);
                num_active_players = num_active_players.saturating_sub(1);
                if let (0, Some(i)) = (num_active_players, idle_inhibitor.as_ref()) {
                    i.destroy();
                    idle_inhibitor = None;
                    conn.roundtrip()
                        .expect("failed to request destruction of idle inhibitor");
                    println!("Idle allowed");
                }
            }
        }
    }
}

#[derive(Default)]
struct State {
    compositor: Option<WlCompositor>,
    surf: Option<WlSurface>,
    idle_inhibit_manager: Option<ZwpIdleInhibitManagerV1>,
}

impl Dispatch<WlRegistry, ()> for State {
    fn event(
        state: &mut State,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } if interface == WL_COMPOSITOR_INTERFACE.name => {
                let compositor = registry.bind::<WlCompositor, _, _>(name, version, qh, ());
                state.surf = Some(compositor.create_surface(qh, ()));
                state.compositor = Some(compositor);
                eprintln!("[{}] {} (v{})", name, interface, version);
            }
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } if interface == ZWP_IDLE_INHIBIT_MANAGER_V1_INTERFACE.name => {
                let idle_inhibit_manager =
                    registry.bind::<ZwpIdleInhibitManagerV1, _, _>(name, version, qh, ());
                state.idle_inhibit_manager = Some(idle_inhibit_manager);
                eprintln!("[{}] {} (v{})", name, interface, version);
            }
            // Don't care
            _ => {}
        }
    }
}

impl Dispatch<WlCompositor, ()> for State {
    fn event(
        _: &mut State,
        _: &WlCompositor,
        _: wl_compositor::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSurface, ()> for State {
    fn event(
        _: &mut State,
        _: &WlSurface,
        _: wl_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpIdleInhibitManagerV1, ()> for State {
    fn event(
        _: &mut State,
        _: &ZwpIdleInhibitManagerV1,
        _: zwp_idle_inhibit_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwpIdleInhibitorV1, ()> for State {
    fn event(
        _: &mut State,
        _: &ZwpIdleInhibitorV1,
        _: zwp_idle_inhibitor_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}
