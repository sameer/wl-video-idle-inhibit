Inhibits idle on [Wayland](https://en.wikipedia.org/wiki/Wayland_(display_server_protocol)) when a video device is open (i.e. you're in a meeting at work).

## Installation

Archlinux users can install [`aur/sway-video-idle-inhibit`](https://aur.archlinux.org/packages/sway-video-idle-inhibit)

## Usage

In Sway config (~/.config/sway/config):

```
# Inhibit idle when a video device is open
exec wl-video-idle-inhibit
```
