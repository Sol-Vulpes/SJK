//! Camera personalization using the existing live, archived camera consumers.
use super::catalog::{Setting, ValueKind};

const fn slider(label: &'static str, cvar: &'static str, min: f64, max: f64, step: f64) -> Setting {
    Setting {
        label,
        cvar,
        kind: ValueKind::Float { min, max, step },
    }
}

pub(super) const ROWS: &[Setting] = &[
    Setting {
        label: "Third-person view",
        cvar: "cg_thirdPerson",
        kind: ValueKind::Bool,
    },
    slider("Field of view", "cg_fov", 70., 130., 1.),
    Setting {
        label: "Widescreen FOV",
        cvar: "cg_fovAspectAdjust",
        kind: ValueKind::Bool,
    },
    slider("Camera distance", "cg_thirdPersonRange", 0., 300., 5.),
    slider("Camera height", "cg_thirdPersonVertOffset", -100., 100., 1.),
    slider(
        "Shoulder offset",
        "cg_thirdPersonHorzOffset",
        -100.,
        100.,
        1.,
    ),
    slider("Orbit angle", "cg_thirdPersonAngle", -180., 180., 5.),
    slider("Pitch offset", "cg_thirdPersonPitchOffset", -90., 90., 1.),
    slider(
        "Camera damping (1 snaps)",
        "cg_thirdPersonCameraDamp",
        0.,
        1.,
        0.05,
    ),
    slider(
        "Target damping (1 snaps)",
        "cg_thirdPersonTargetDamp",
        0.,
        1.,
        0.05,
    ),
    slider("Damping FPS (0 stock)", "cg_cameraFPS", 0., 333., 1.),
    slider("Prediction smoothing (ms)", "cg_errorDecay", 0., 500., 10.),
    slider("Vertical bob", "cg_bobUp", 0., 0.02, 0.001),
    slider("First-person pitch bob", "cg_bobPitch", 0., 0.01, 0.0005),
    slider("First-person roll bob", "cg_bobRoll", 0., 0.01, 0.0005),
    Setting {
        label: "Effect camera shake",
        cvar: "cg_screenShake",
        kind: ValueKind::Integer {
            min: 0,
            max: 2,
            step: 1,
        },
    },
    slider(
        "View-model FOV (0 uses world FOV)",
        "cg_fovViewmodel",
        0.,
        130.,
        1.,
    ),
    Setting {
        label: "View-model lowering",
        cvar: "cg_fovViewmodelAdjust",
        kind: ValueKind::Bool,
    },
];
