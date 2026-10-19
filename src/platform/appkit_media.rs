//! macOS media playback via AVFoundation [`AVPlayer`] + [`AVPlayerView`].

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

use objc2::rc::Retained;
use objc2_av_foundation::{AVPlayer, AVPlayerItem};
use objc2_av_kit::AVPlayerView;
use objc2_core_media::CMTime;
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString, NSURL};

use crate::adv::media_ctrl::MediaState;
use crate::core::geometry::Rect;

use super::{child_view_for_handle, create_panel, NativeHandle};

thread_local! {
    static MEDIA_HOSTS: RefCell<HashMap<NativeHandle, MediaHost>> =
        RefCell::new(HashMap::new());
}

struct MediaHost {
    player: Retained<AVPlayer>,
    player_view: Retained<AVPlayerView>,
    loaded: bool,
    state: MediaState,
}

fn cmtime_to_ms(time: CMTime) -> Option<u64> {
    if time.timescale == 0 {
        return None;
    }
    Some(((time.value as f64 / time.timescale as f64) * 1000.0).round() as u64)
}

pub fn media_ctrl_create(parent: NativeHandle, rect: Rect) -> NativeHandle {
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    let handle = create_panel(parent, rect);
    let player = unsafe { AVPlayer::new(mtm) };
    let player_view = unsafe { AVPlayerView::new(mtm) };
    unsafe {
        player_view.setPlayer(Some(&player));
        player_view.setControlsStyle(objc2_av_kit::AVPlayerViewControlsStyle::None);
    }

    if let Some(container) = child_view_for_handle(handle) {
        player_view.setHidden(true);
        player_view.setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0)));
        container.setHidden(true);
        container.addSubview(&player_view);
    }

    MEDIA_HOSTS.with(|m| {
        m.borrow_mut().insert(
            handle,
            MediaHost {
                player,
                player_view,
                loaded: false,
                state: MediaState::Stopped,
            },
        );
    });
    handle
}

pub fn media_ctrl_load(handle: NativeHandle, path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("file not found: {}", path.display()));
    }
    let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let path_str = abs.to_str().ok_or_else(|| "non-UTF-8 path".to_string())?;
    let mtm = MainThreadMarker::new().expect("AppKit requires the main thread");
    MEDIA_HOSTS.with(|m| {
        let mut map = m.borrow_mut();
        let host = map
            .get_mut(&handle)
            .ok_or_else(|| "invalid media control".to_string())?;
        let url = NSURL::fileURLWithPath(&NSString::from_str(path_str));
        let item = unsafe { AVPlayerItem::playerItemWithURL(&url, mtm) };
        unsafe {
            host.player.replaceCurrentItemWithPlayerItem(Some(&item));
        }
        host.loaded = true;
        host.state = MediaState::Stopped;
        Ok(())
    })
}

pub fn media_ctrl_play(handle: NativeHandle) -> Result<(), String> {
    MEDIA_HOSTS.with(|m| {
        let mut map = m.borrow_mut();
        let host = map
            .get_mut(&handle)
            .ok_or_else(|| "invalid media control".to_string())?;
        if !host.loaded {
            return Err("no media loaded".to_string());
        }
        unsafe {
            host.player.play();
        }
        host.state = MediaState::Playing;
        Ok(())
    })
}

pub fn media_ctrl_pause(handle: NativeHandle) -> Result<(), String> {
    MEDIA_HOSTS.with(|m| {
        let mut map = m.borrow_mut();
        let host = map
            .get_mut(&handle)
            .ok_or_else(|| "invalid media control".to_string())?;
        unsafe {
            host.player.pause();
        }
        host.state = MediaState::Paused;
        Ok(())
    })
}

pub fn media_ctrl_stop(handle: NativeHandle) -> Result<(), String> {
    MEDIA_HOSTS.with(|m| {
        let mut map = m.borrow_mut();
        let host = map
            .get_mut(&handle)
            .ok_or_else(|| "invalid media control".to_string())?;
        unsafe {
            host.player.pause();
            host.player.seekToTime(CMTime::with_seconds(0.0, 600));
        }
        host.state = MediaState::Stopped;
        Ok(())
    })
}

pub fn media_ctrl_close(handle: NativeHandle) -> Result<(), String> {
    MEDIA_HOSTS.with(|m| {
        if let Some(host) = m.borrow_mut().get_mut(&handle) {
            unsafe {
                host.player.replaceCurrentItemWithPlayerItem(None);
            }
            host.loaded = false;
            host.state = MediaState::Stopped;
        }
        Ok(())
    })
}

pub fn media_ctrl_position_ms(handle: NativeHandle) -> Option<u64> {
    MEDIA_HOSTS.with(|m| {
        m.borrow()
            .get(&handle)
            .and_then(|host| unsafe { cmtime_to_ms(host.player.currentTime()) })
    })
}

pub fn media_ctrl_length_ms(handle: NativeHandle) -> Option<u64> {
    MEDIA_HOSTS.with(|m| {
        m.borrow().get(&handle).and_then(|host| unsafe {
            host.player
                .currentItem()
                .map(|item| item.duration())
                .and_then(cmtime_to_ms)
        })
    })
}

pub fn media_ctrl_seek_ms(handle: NativeHandle, ms: u64) -> Result<(), String> {
    MEDIA_HOSTS.with(|m| {
        let map = m.borrow();
        let host = map
            .get(&handle)
            .ok_or_else(|| "invalid media control".to_string())?;
        unsafe {
            let time = CMTime::with_seconds(ms as f64 / 1000.0, 600);
            let zero = CMTime::new(0, 1);
            host.player
                .seekToTime_toleranceBefore_toleranceAfter(time, zero, zero);
        }
        Ok(())
    })
}

pub fn media_ctrl_state(handle: NativeHandle) -> MediaState {
    MEDIA_HOSTS.with(|m| {
        m.borrow()
            .get(&handle)
            .map(|h| h.state)
            .unwrap_or(MediaState::Stopped)
    })
}

pub fn media_ctrl_destroy(handle: NativeHandle) {
    MEDIA_HOSTS.with(|m| m.borrow_mut().remove(&handle));
}
