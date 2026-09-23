#!/usr/bin/env python3
"""robot-sim bridge: lock-step JSON protocol between the Rust safety gate
(swf-cli) and the simulator backend.

Protocol messages travel alone on stdout, one JSON object per line.
All diagnostics go to stderr. Nothing else may write to stdout.

Backends:
  robosuite : the real SIL path — robosuite 1.5.2 / MuJoCo, Panda + Lift,
              pinned BASIC composite controller (Linux x86-64, CPython 3.12).
  mock      : deterministic kinematic stand-in with NO physics engine. It
              exists so the protocol, gate and verifier run anywhere (CI,
              presenter laptop). Mock episodes are labeled in every message
              and are NOT simulation results.

Staleness model: the observation stream is delayed by N ticks. Simulation
time advances through explicitly labeled hold steps while the delivered
observation lags — a real historical observation, not a paused clock.
"""

import argparse
import json
import sys
import time

from scripted_controller import SEGMENTS, segment_target, osc_action, LIFT_DELTA_M

TICK_NS = 50_000_000          # 20 Hz control rate
NS_PER_MS = 1_000_000
PROTOCOL_VERSION = 1
TABLE_TOP_Z = 0.82            # table surface height used by the mock
LIFT_SUCCESS_DELTA_M = 0.10   # explicit height condition alongside env check


def emit(obj: dict) -> None:
    """Send one protocol message. stdout is protocol-only."""
    try:
        sys.stdout.write(json.dumps(obj) + "\n")
        sys.stdout.flush()
    except BrokenPipeError:
        sys.exit(2)  # gate is gone


def log(msg: str) -> None:
    sys.stderr.write(f"[bridge] {msg}\n")
    sys.stderr.flush()


# --------------------------------------------------------------------------
# Backends
# --------------------------------------------------------------------------

class BackendBase:
    backend = "base"
    backend_label = "base"

    def __init__(self, placement: dict):
        self.tick = 0
        self.history = []  # observation per tick, index == tick

    def capture_obs(self) -> dict:
        raise NotImplementedError

    def hold(self) -> None:
        """Advance physics one tick with no task action (labeled hold)."""
        raise NotImplementedError

    def apply(self, segment: str, target: dict) -> None:
        """Apply one permitted task action; advances one or more ticks."""
        raise NotImplementedError

    def cube_height(self) -> float:
        raise NotImplementedError

    def success(self) -> bool:
        raise NotImplementedError

    def _record(self) -> None:
        self.history.append(self.capture_obs())


class MockBackend(BackendBase):
    """Kinematic stand-in. Deterministic, stdlib-only, clearly labeled."""

    backend = "mock"
    backend_label = "mock (kinematic stand-in; NOT robosuite SIL)"

    def __init__(self, placement: dict):
        super().__init__(placement)
        self.cube = [float(placement.get("x", 0.0)), float(placement.get("y", 0.0)), TABLE_TOP_Z]
        self.eef = [0.0, 0.0, TABLE_TOP_Z + 0.30]
        self.grasped = False
        self._record()

    def capture_obs(self) -> dict:
        return {
            "observation_id": f"obs-{self.tick}",
            "capture_time_ns": self.tick * TICK_NS,
            "cube_pos": list(self.cube),
            "eef_pos": list(self.eef),
        }

    def hold(self) -> None:
        self.tick += 1
        self._record()

    def apply(self, segment: str, target: dict) -> None:
        # The target was computed from the DELIVERED observation, which in
        # stale scenarios is historical — that is the point of the demo.
        self.eef = list(target["eef"])
        if segment == "grasp":
            self.grasped = True
        if segment == "lift" and self.grasped:
            self.cube = [self.eef[0], self.eef[1], self.eef[2] - 0.02]
        self.tick += 1
        self._record()

    def cube_height(self) -> float:
        return self.cube[2]

    def success(self) -> bool:
        return self.cube[2] >= TABLE_TOP_Z + LIFT_SUCCESS_DELTA_M - 1e-9


class RobosuiteBackend(BackendBase):
    """Real SIL path: robosuite Lift, Panda, pinned BASIC controller config."""

    backend = "robosuite"

    def __init__(self, placement: dict, video_dir: str | None = None):
        super().__init__(placement)
        import robosuite as suite  # noqa: PLC0415
        from robosuite.controllers import load_composite_controller_config  # noqa: PLC0415

        self._np = __import__("numpy")
        self._video_dir = video_dir
        self._frames = []

        controller_config = load_composite_controller_config(robot="Panda")  # pinned BASIC
        self.env = suite.make(
            env_name="Lift",
            robots="Panda",
            controller_configs=controller_config,
            has_renderer=False,
            has_offscreen_renderer=video_dir is not None,
            render_camera="frontview",
            use_camera_obs=False,
            control_freq=int(1e9 / TICK_NS),
            horizon=10_000,
            hard_reset=True,
        )
        self.env.reset()
        self._cube_body = self._resolve_body(["cube_main", "cube"])
        self._eef_body = self._resolve_body(
            ["gripper0_right_eef", "gripper0_right_gripper", "gripper0_right_hand"]
        )
        self._pin_placement(placement)
        self.backend_label = (
            f"robosuite {suite.__version__} / mujoco {__import__('mujoco').__version__}"
        )
        log(f"robosuite action spec pinned: {self.env.action_spec[0].shape}")
        self._record()

    def _resolve_body(self, candidates) -> str:
        names = set(self.env.sim.model.body_names)
        for c in candidates:
            if c in names:
                return c
        raise RuntimeError(f"none of {candidates} in model bodies")

    def _pin_placement(self, placement: dict) -> None:
        """Stable, rehearsed cube placement: default pose plus fixed offsets."""
        np = self._np
        addr = self.env.sim.model.get_joint_qpos_addr("cube_joint0")
        if isinstance(addr, tuple):  # robosuite returns (start, end)
            addr = addr[0]
        qpos = self.env.sim.data.qpos.copy()
        qpos[addr + 0] += float(placement.get("x", 0.0))
        qpos[addr + 1] += float(placement.get("y", 0.0))
        self.env.sim.data.qpos[:] = qpos
        self.env.sim.forward()
        self._cube_z0 = float(self.env.sim.data.get_body_xpos(self._cube_body)[2])

    def _eef_pos(self):
        return self._np.array(self.env.sim.data.get_body_xpos(self._eef_body))

    def capture_obs(self) -> dict:
        np = self._np
        cube = np.array(self.env.sim.data.get_body_xpos(self._cube_body))
        return {
            "observation_id": f"obs-{self.tick}",
            "capture_time_ns": self.tick * TICK_NS,
            "cube_pos": [float(v) for v in cube],
            "eef_pos": [float(v) for v in self._eef_pos()],
        }

    def _step(self, action) -> None:
        self.env.step(action)
        self.tick += 1
        if self._video_dir is not None:
            frame = self.env.sim.render(width=640, height=480, camera_name="frontview")
            self._frames.append(frame[::-1])  # MuJoCo renders bottom-up
        self._record()

    def hold(self) -> None:
        self._step([0.0] * 6 + [-1.0])  # zero arm delta, gripper open

    def apply(self, segment: str, target: dict) -> None:
        max_steps = {"approach": 60, "descend": 60, "grasp": 20, "lift": 80}[segment]
        for _ in range(max_steps):
            action = osc_action(self._eef_pos(), target)
            self._step(action)
            err = max(abs(t - e) for t, e in zip(target["eef"], self._eef_pos()))
            if segment == "grasp":
                if self.tick % 10 == 0:
                    break
            elif err < 0.005:
                break

    def cube_height(self) -> float:
        return float(self.env.sim.data.get_body_xpos(self._cube_body)[2])

    def success(self) -> bool:
        # Environment's own success check plus the explicit height condition.
        env_success = bool(self.env._check_success())
        height_ok = self.cube_height() >= self._cube_z0 + LIFT_SUCCESS_DELTA_M - 1e-9
        return env_success and height_ok

    def finalize_video(self) -> None:
        if self._video_dir and self._frames:
            import os  # noqa: PLC0415
            import imageio.v2 as imageio  # noqa: PLC0415

            os.makedirs(self._video_dir, exist_ok=True)
            path = os.path.join(self._video_dir, "episode.mp4")
            imageio.mimsave(path, self._frames, fps=20)
            log(f"wrote {path}")


# --------------------------------------------------------------------------
# Episode loop
# --------------------------------------------------------------------------

REJECTION_TO_OUTCOME = {
    "emergency_stop": "emergency_stop",
    "invalid_timestamp": "rejected_invalid",
    "stale_perception": "rejected_stale",
}


def read_decision(expected_action_id: str, expected_tick: int) -> dict:
    line = sys.stdin.readline()
    if not line:
        log("gate closed stdin; exiting")
        sys.exit(2)
    decision = json.loads(line)
    # Approvals are valid only for the exact action and tick.
    if (
        decision.get("type") != "decision"
        or decision.get("action_id") != expected_action_id
        or decision.get("simulation_tick") != expected_tick
    ):
        return {"type": "mismatch", "raw": decision}
    return decision


def run_episode(args, scenario: dict, backend: BackendBase) -> None:
    run_id, episode_id = args.run_id, args.episode_id
    staleness_ms = int(scenario.get("staleness_ms", 0))
    delay_ticks = staleness_ms * NS_PER_MS // TICK_NS
    stop_at = scenario.get("stop_at_segment")

    emit(
        {
            "type": "hello",
            "backend": backend.backend,
            "backend_label": backend.backend_label,
            "version": PROTOCOL_VERSION,
            "simulation_tick_ns": TICK_NS,
        }
    )

    stall_ms = int(scenario.get("stall_before_proposal_ms", 0))
    if stall_ms:
        log(f"scenario requests a {stall_ms} ms pre-proposal stall (protocol-timeout fixture)")
        time.sleep(stall_ms / 1000.0)

    # Staleness injection: advance simulated time through labeled hold steps
    # while the observation stream lags by `delay_ticks`.
    for _ in range(delay_ticks):
        backend.hold()
        emit(
            {
                "type": "hold",
                "simulation_tick": backend.tick,
                "simulation_time_ns": backend.tick * TICK_NS,
                "reason": "staleness-injection",
            }
        )

    def end_episode(success: bool, reason: str) -> None:
        emit(
            {
                "type": "episode_end",
                "success": success,
                "reason": reason,
                "cube_height_m": round(backend.cube_height(), 4),
                "ticks": backend.tick,
            }
        )
        if hasattr(backend, "finalize_video"):
            backend.finalize_video()

    for segment in SEGMENTS:
        tick = backend.tick
        delivered = backend.history[max(0, tick - delay_ticks)]
        action_id = f"{episode_id}-{segment}-{tick}"
        proposal = {
            "type": "proposal",
            "run_id": run_id,
            "episode_id": episode_id,
            "action_id": action_id,
            "simulation_tick": tick,
            "simulation_time_ns": tick * TICK_NS,
            "observation": {
                "observation_id": delivered["observation_id"],
                "capture_time_ns": delivered["capture_time_ns"],
            },
            "simulated_stop": stop_at == segment,
            "proposed_action": {"kind": "pickup", "target": "cube", "segment": segment},
        }
        emit(proposal)

        decision = read_decision(action_id, tick)
        if decision.get("type") == "mismatch":
            emit(
                {
                    "type": "outcome",
                    "action_id": action_id,
                    "simulation_tick": tick,
                    "dispatched": False,
                    "action_kind": "task",
                    "note": "approval-mismatch",
                }
            )
            end_episode(False, "protocol_violation")
            return

        if decision.get("decision") == "reject":
            reason = decision.get("reason", "unknown")
            emit(
                {
                    "type": "outcome",
                    "action_id": action_id,
                    "simulation_tick": tick,
                    "dispatched": False,
                    "action_kind": "task",
                    "note": f"rejected:{reason}",
                }
            )
            emit(
                {
                    "type": "hold",
                    "simulation_tick": backend.tick,
                    "simulation_time_ns": backend.tick * TICK_NS,
                    "reason": f"rejection:{reason}",
                }
            )
            end_episode(False, REJECTION_TO_OUTCOME.get(reason, "rejected_unknown"))
            return

        # Permitted: dispatch the rehearsed segment computed from the
        # DELIVERED observation (historical in stale scenarios).
        target = segment_target(delivered, segment)
        backend.apply(segment, target)
        emit(
            {
                "type": "outcome",
                "action_id": action_id,
                "simulation_tick": tick,
                "dispatched": True,
                "action_kind": "task",
                "note": f"segment:{segment}",
            }
        )

    success = backend.success()
    end_episode(success, "cube_lifted" if success else "task_incomplete")


def main() -> None:
    parser = argparse.ArgumentParser(description="robot-sim lock-step bridge")
    parser.add_argument("--scenario", required=True)
    parser.add_argument("--backend", choices=["mock", "robosuite"], default="mock")
    parser.add_argument("--run-id", required=True)
    parser.add_argument("--episode-id", required=True)
    parser.add_argument("--video-dir", default=None)
    args = parser.parse_args()

    with open(args.scenario, "r", encoding="utf-8") as f:
        scenario = json.load(f)

    placement = scenario.get("placement", {})
    if args.backend == "robosuite":
        backend = RobosuiteBackend(placement, video_dir=args.video_dir)
    else:
        backend = MockBackend(placement)

    log(
        f"scenario={scenario.get('name')} backend={backend.backend_label} "
        f"staleness_ms={scenario.get('staleness_ms', 0)}"
    )
    run_episode(args, scenario, backend)


if __name__ == "__main__":
    main()
