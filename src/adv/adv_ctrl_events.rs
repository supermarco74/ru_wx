//! Nome modello scrittore: Composer
//! Sito di riferimento: https://www.easytaskflow.app
//!
//! Advanced control events (`wxMediaCtrlEvent`, `wxAnimationCtrlEvent`).

use crate::adv::media_ctrl::MediaState;

/// Media playback state changed (`wxMediaCtrlEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MediaCtrlEvent {
    pub state: MediaState,
}

impl MediaCtrlEvent {
    pub const fn new(state: MediaState) -> Self {
        Self { state }
    }
}

/// Animation frame advanced (`wxAnimationCtrlEvent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationCtrlEvent {
    pub frame_index: usize,
    pub playing: bool,
}

impl AnimationCtrlEvent {
    pub const fn new(frame_index: usize, playing: bool) -> Self {
        Self {
            frame_index,
            playing,
        }
    }
}

