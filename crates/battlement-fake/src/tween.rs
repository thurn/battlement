//! Instant tween endpoint selection.

use battlement::{Tween, TweenRepeat};

pub(crate) fn final_factor(tween: Tween) -> f64 {
    match tween.repeat {
        TweenRepeat::Once | TweenRepeat::Forever(_) => 1.0,
        TweenRepeat::Count {
            additional_traversals,
            mode,
        } => match mode {
            battlement::RepeatMode::Restart => 1.0,
            battlement::RepeatMode::PingPong => {
                if additional_traversals % 2 == 1 {
                    0.0
                } else {
                    1.0
                }
            }
        },
    }
}

pub(crate) fn scalar(start: f64, target: f64, tween: Tween) -> f64 {
    start + (target - start) * final_factor(tween)
}

pub(crate) fn vector(
    start: battlement::Vector3,
    target: battlement::Vector3,
    tween: Tween,
) -> battlement::Vector3 {
    battlement::Vector3::new(
        scalar(start.x, target.x, tween),
        scalar(start.y, target.y, tween),
        scalar(start.z, target.z, tween),
    )
}

pub(crate) fn rgb(
    start: battlement::RgbColor,
    target: battlement::RgbColor,
    tween: Tween,
) -> battlement::RgbColor {
    battlement::RgbColor {
        r: scalar(start.r, target.r, tween),
        g: scalar(start.g, target.g, tween),
        b: scalar(start.b, target.b, tween),
    }
}

pub(crate) fn color(
    start: battlement::Color,
    target: battlement::Color,
    tween: Tween,
) -> battlement::Color {
    battlement::Color {
        r: scalar(start.r, target.r, tween),
        g: scalar(start.g, target.g, tween),
        b: scalar(start.b, target.b, tween),
        a: scalar(start.a, target.a, tween),
    }
}
