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
brew install phgi/tenki/tenki
```

Nothing else to install — `tenki` is a single self-contained binary. Homebrew
pulls in Rust only to compile it, then throws it away.

<details>
<summary>Publishing the formula (one-time, for the author)</summary>

The formula in `Formula/tenki.rb` is ready except for the repository owner and
the release checksum:

1. Push this repo to GitHub and replace the placeholder:
   ```sh
   sed -i '' 's/phgi/your-github-username/g' Formula/tenki.rb README.md
   ```
2. Tag a release: `git tag v0.1.0 && git push --tags`
3. Fill in the checksum:
   ```sh
   curl -sL https://github.com/phgi/tenki/archive/refs/tags/v0.1.0.tar.gz | shasum -a 256
   ```
4. Publish it in a tap named `homebrew-tenki` (a repo containing
   `Formula/tenki.rb`), which is what makes `brew install phgi/tenki/tenki`
   work.

Before any of that, `brew install --HEAD --build-from-source ./Formula/tenki.rb`
installs straight from the checked-out source.

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
