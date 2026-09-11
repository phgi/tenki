# tenki 天気

An animated terminal weather app. It finds you by IP, pulls the current
conditions from [Open-Meteo](https://open-meteo.com), and fills the terminal
with vaporwave-pastel ASCII art — rain that falls, snow that drifts, waves that
roll — with a detail panel one keystroke away.

```
     ~~~~~~~~~~~~~~~~               ██  |     ██████  |   ████        ~~~ |
                                    ██        ██████      ████              |
                 ~~|~~~~~~~       ████      ██      ██  ██    ██      ~~~~
             ~~~~          |~~    ████      ██      ██  ██    ██          ~~~~
         ~~~~                       ██   |    ██████      ████                ~~~~
  |  ~~~~                           ██        ██████      ████ '                  ~~~~
~~~~~                               ██      ██      ██                                ~~~~~~~~~~~~
         '                         |██      ██   '  ██            '
   ~~~~~~~~~~~       '            ██████      ██████     |    |       |    '           ~~~~~~~~~~~
~~~           ~'~~                ██████      ██████    '                 '|       ~~~~           ~~
  '  '          ' ~~~                                                           ~~~
        '            ~~~~                      ☂ RAIN                       ~~|~
               Asahi, Chiba  ·  updated 0s ago  ·  [d] details  [r] refresh  [q] quit
```

Press `d` for the numbers:

```
╭──────────────────── Asahi, Chiba ────────────────────╮
│                                                      │
│🌡   Temperature       18.4°C                          │
│🤔   Feels like        17.1°C                          │
│💧   Humidity          72%                             │
│🌬   Wind              14 km/h SW                      │
│⏲   Pressure          1013 hPa                        │
│😎   UV index          4.2                             │
│🌂   Chance of rain    65%                             │
│                                                      │
│🌅   Sunrise           05:21                           │
│🌇   Sunset            18:04                           │
│                                                      │
╰────────────────── press d to close ──────────────────╯
```

## Install

```sh
brew tap phgi/tenki https://github.com/phgi/tenki
brew install phgi/tenki/tenki
```

That downloads a prebuilt binary — a single self-contained ~2.6 MB executable
with no runtime dependencies. No Rust toolchain, no compiling, no crate
downloads; it takes a couple of seconds.

To build current `main` from source instead, use `brew install --HEAD
phgi/tenki/tenki`. That route *does* install a Rust toolchain as a build
dependency, and Homebrew keeps it afterwards — `brew autoremove` drops it
again once the install is done.

Two things that trip people up:

- **The tap step is not optional.** Homebrew 4 refuses to install from a loose
  formula path (`Error: Homebrew requires formulae to be in a tap`).
- **No separate `homebrew-tenki` repo is needed.** `brew tap` accepts an
  explicit URL, so this repo serves as its own tap.

<details>
<summary>Cutting a release (for the author)</summary>

Releases are automated by `.github/workflows/release.yml`. Pushing a `v*` tag
builds both macOS binaries (arm64 natively, x86_64 cross-compiled from the same
runner), publishes them as release assets, and commits a regenerated
`Formula/tenki.rb` pointing at them with real checksums.

```sh
# 1. bump `version` in Cargo.toml, then refresh the lockfile's own entry
cargo update --offline --package tenki --precise 0.1.1

# 2. commit both — a lockfile left behind breaks every `--locked` build
git commit -am "release: v0.1.1" && git push

# 3. tag the commit you just pushed
git tag v0.1.1 && git push --tags
```

The tag must match the version in `Cargo.toml` — the workflow fails fast if it
doesn't. `Cargo.lock` records the version of `tenki` itself, so it has to move
with it; the workflow re-syncs that entry defensively, but committing it keeps
local `--locked` builds working too. An existing tag can be rebuilt from the
Actions tab via the `workflow_dispatch` trigger.

</details>

### From source

```sh
cargo install --path .
```

## Usage

```sh
tenki                          # weather where you are
tenki --lat 35.72 --lon 140.65 # somewhere else
```

| Key                    | Action                  |
| ---------------------- | ----------------------- |
| `d` / `i`              | toggle the detail panel |
| `r`                    | refresh now             |
| `q` / `Esc` / `Ctrl-C` | quit                    |

The display fills whatever space the terminal has and re-lays itself out when
you resize. Weather refreshes every 10 minutes; if a refresh fails the last
good reading stays on screen.

## How it works

|                       |                                                                                                                                               |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `src/net/`            | IP geolocation ([ipwho.is](https://ipwho.is), falling back to ip-api.com) and the Open-Meteo forecast call, plus the WMO weather-code mapping |
| `src/ui/theme.rs`     | the pastel palette and gradient maths                                                                                                         |
| `src/ui/canvas.rs`    | the full-screen widget: wave background, hero temperature, particles                                                                          |
| `src/ui/bigfont.rs`   | a hand-rolled 5×5 block font for the big temperature                                                                                          |
| `src/ui/particles.rs` | per-condition particle system (rain, snow, drizzle, fog)                                                                                      |
| `src/ui/detail.rs`    | the `d` panel                                                                                                                                 |

Neither service needs an API key. Nothing is stored and nothing is sent
anywhere except the two requests above.

To see the artwork without a terminal:

```sh
cargo test -- --ignored --nocapture
```

## License

MIT
