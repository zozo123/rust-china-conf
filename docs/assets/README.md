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
