"""Rehearsed motion proposals for the Panda Lift task.

No learning in the live path: a fixed approach/descend/grasp/lift sequence
computed from the (possibly stale) simulated observation attached to the
proposal. Both backends share this module so the rehearsed logic is identical
in mock and robosuite modes.
"""

SEGMENTS = ["approach", "descend", "grasp", "lift"]

# Rehearsed offsets relative to the cube position (metres).
HOVER_OFFSET_M = 0.10   # approach: hover above the cube
GRASP_OFFSET_M = 0.02   # descend: fingers around the cube
LIFT_DELTA_M = 0.20     # lift: carry the cube upward


def segment_target(obs: dict, segment: str) -> dict:
    """Target end-effector pose + gripper for one segment, from an observation.

    `obs` carries cube_pos/eef_pos captured at observation_capture_time_ns —
    in stale scenarios this is a historical observation by construction.
    """
    cube = obs["cube_pos"]
    if segment == "approach":
        return {"eef": [cube[0], cube[1], cube[2] + HOVER_OFFSET_M], "gripper": -1}
    if segment == "descend":
        return {"eef": [cube[0], cube[1], cube[2] + GRASP_OFFSET_M], "gripper": -1}
    if segment == "grasp":
        return {"eef": [cube[0], cube[1], cube[2] + GRASP_OFFSET_M], "gripper": 1}
    if segment == "lift":
        return {"eef": [cube[0], cube[1], cube[2] + LIFT_DELTA_M], "gripper": 1}
    raise ValueError(f"unknown segment {segment!r}")


def osc_action(eef_pos, target: dict, pos_scale: float = 0.05) -> list:
    """Normalized OSC pose action (dx, dy, dz, ax, ay, az, gripper).

    Position deltas are clipped to +/-pos_scale per control step, matching the
    pinned Panda BASIC controller's input range of [-1, 1] scaled by its
    output limits. Rotation is held at zero (rehearsed vertical approach).
    """
    t = target["eef"]
    delta = [
        max(-1.0, min(1.0, (t[i] - eef_pos[i]) / pos_scale)) for i in range(3)
    ]
    return [delta[0], delta[1], delta[2], 0.0, 0.0, 0.0, float(target["gripper"])]
