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
brew install --HEAD phgi/tenki/tenki
```

Nothing else to install — `tenki` is a single self-contained binary with no
runtime dependencies. Homebrew pulls in Rust only to compile it, then throws
it away.

Two things that trip people up:

- **The tap step is not optional.** Homebrew 4 refuses to install from a loose
  formula path (`Error: Homebrew requires formulae to be in a tap`).
- **No separate `homebrew-tenki` repo is needed.** `brew tap` accepts an
  explicit URL, so this repo serves as its own tap.

`--HEAD` builds whatever is on `main`. Plain `brew install phgi/tenki/tenki`
needs a tagged release first:

<details>
<summary>Cutting a release (for the author)</summary>

1. Tag it:
   ```sh
   git tag v0.1.0 && git push --tags
   ```
2. Get the tarball checksum:
   ```sh
   curl -sL https://github.com/phgi/tenki/archive/refs/tags/v0.1.0.tar.gz | shasum -a 256
   ```
3. Paste it over `REPLACE_WITH_TARBALL_SHA256` in `Formula/tenki.rb`, then
   commit and push.

Users on an older checkout may need `brew update` before the new version shows
up.

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
