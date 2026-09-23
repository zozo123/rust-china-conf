# Website media

`robot-lift.gif` contains real frames from robosuite 1.5.2 / MuJoCo 3.9.0.
The episode was run through `swf-cli`, so each approach, descend, grasp and
lift segment was authorized by the Rust safety gate before the simulator
executed it.

Regenerate it from the pinned simulator environment:

```bash
scripts/robot-demo/record-gif.sh
```

The script records `fresh_lift`, converts the resulting MP4 to a 512×384,
12 fps GIF, and removes its temporary evidence directory. It requires
`ffmpeg`; `gifsicle` is optional and used for additional optimization.

This GIF demonstrates simulator motion. The digest-bound conference
evidence remains the committed run `ec2-e2e-20260923-160725` under
`docs/examples/`; regenerating website media does not replace that evidence.

## The loop walkthrough

`validation-loop.mp4` (plus `validation-loop-poster.png`) is what the website
plays. `validation-loop.gif` is the same footage, kept because GitHub's README
renderer will not play a committed video file.

```bash
scripts/robot-demo/record-loop-gif.sh
```

Both come from `docs/demo/loop.html`, which renders a deterministic state at
`?frame=N`, so the media is reproducible instead of hand-assembled.

The site uses the video rather than the GIF for accessibility, not size — the
MP4 is actually larger (417 KB against 285 KB), because H.264 handles sharp
terminal text less efficiently than a GIF palette does. The reason to prefer it
is that the walkthrough runs 19 seconds: motion that long needs a pause
control, and a muted loop should not start for a visitor who has asked for
reduced motion. A GIF can do neither.
