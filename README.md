# Workshop Timer

Small always-on-top overlay that counts down workshop segments defined in `segments.yaml`. Built with `eframe`/`egui`.

## Run

```
make run        # debug build, launches the overlay
make release    # optimized build at target/release/workshop-timer
```

The overlay loads `segments.yaml` from the current working directory (or next to the executable). Right-click the bar for the menu (agenda, edit YAML, reload, load, close).

## Editing `segments.yaml`

Each entry has `name`, `duration`, and optional `description`. Duration accepts `25m`, `1h30m`, `90s`, or `mm:ss` / `hh:mm:ss`.

**Keep `name` short — about 12 characters or fewer.** The overlay is a narrow always-on-top bar (~360 px) and renders each segment as `"N/<total> <name>"` with truncation. Anything longer gets visibly cut off. Put the full phrasing in `description` — it shows on hover and in the agenda window.

Good:

```yaml
- name: FAQ Agent
  duration: 25m
  description: Segment 2 — load FAQ, index with minsearch, wrap Responses API loop
```

Too long for the bar:

```yaml
- name: Build the FAQ Agent      # gets truncated
  duration: 25m
```
