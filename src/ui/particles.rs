use rand::rngs::ThreadRng;
use rand::Rng;

use crate::net::model::Condition;

#[derive(Clone, Copy)]
pub struct Particle {
    pub x: f64,
    pub y: f64,
    pub vx: f64,
    pub vy: f64,
    pub glyph: char,
    pub brightness: f64,
}

fn density_for(condition: Condition, width: f64, height: f64) -> usize {
    let area = (width * height).max(1.0);
    let n = match condition {
        Condition::Rain | Condition::RainShowers | Condition::Freezing => area / 40.0,
        Condition::Thunderstorm => area / 32.0,
        Condition::Drizzle => area / 90.0,
        Condition::Snow | Condition::SnowShowers => area / 55.0,
        Condition::Fog => area / 100.0,
        Condition::Clear | Condition::PartlyCloudy | Condition::Overcast => 0.0,
    };
    (n as usize).min(500)
}

fn spawn_particle(condition: Condition, width: f64, height: f64, rng: &mut ThreadRng, anywhere_y: bool) -> Particle {
    let w = width.max(1.0);
    let h = height.max(1.0);
    let x = rng.gen_range(0.0..w);
    let y = if anywhere_y { rng.gen_range(0.0..h) } else { -1.0 };

    match condition {
        Condition::Rain | Condition::RainShowers | Condition::Thunderstorm | Condition::Freezing => Particle {
            x,
            y,
            vx: -0.3,
            vy: rng.gen_range(1.4..2.4),
            glyph: if rng.gen_bool(0.5) { '|' } else { '\'' },
            brightness: rng.gen_range(0.6..1.0),
        },
        Condition::Drizzle => Particle {
            x,
            y,
            vx: -0.1,
            vy: rng.gen_range(0.6..1.0),
            glyph: '.',
            brightness: rng.gen_range(0.5..0.8),
        },
        Condition::Snow | Condition::SnowShowers => Particle {
            x,
            y,
            vx: rng.gen_range(-0.3..0.3),
            vy: rng.gen_range(0.2..0.5),
            glyph: if rng.gen_bool(0.5) { '*' } else { '.' },
            brightness: rng.gen_range(0.7..1.0),
        },
        Condition::Fog => Particle {
            x,
            y: rng.gen_range(0.0..h),
            vx: rng.gen_range(0.1..0.3),
            vy: 0.0,
            glyph: '~',
            brightness: rng.gen_range(0.25..0.55),
        },
        Condition::Clear | Condition::PartlyCloudy | Condition::Overcast => Particle {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            glyph: ' ',
            brightness: 0.0,
        },
    }
}

pub struct ParticleSystem {
    particles: Vec<Particle>,
    condition: Condition,
    width: f64,
    height: f64,
}

impl ParticleSystem {
    pub fn new(condition: Condition, width: u16, height: u16) -> Self {
        let mut sys = ParticleSystem {
            particles: Vec::new(),
            condition,
            width: width as f64,
            height: height as f64,
        };
        sys.populate();
        sys
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width as f64;
        self.height = height as f64;
        self.populate();
    }

    pub fn set_condition(&mut self, condition: Condition) {
        if condition != self.condition {
            self.condition = condition;
            self.populate();
        }
    }

    fn populate(&mut self) {
        let mut rng = rand::thread_rng();
        let n = density_for(self.condition, self.width, self.height);
        self.particles = (0..n)
            .map(|_| spawn_particle(self.condition, self.width, self.height, &mut rng, true))
            .collect();
    }

    pub fn tick(&mut self, dt: f64) {
        let mut rng = rand::thread_rng();
        let (condition, w, h) = (self.condition, self.width, self.height);
        for p in self.particles.iter_mut() {
            p.x += p.vx * dt * 20.0;
            p.y += p.vy * dt * 20.0;
            if condition == Condition::Fog {
                if p.x > w {
                    p.x = 0.0;
                }
            } else if p.y > h || p.x < 0.0 || p.x > w {
                *p = spawn_particle(condition, w, h, &mut rng, false);
            }
        }
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_sky_has_no_particles() {
        let sys = ParticleSystem::new(Condition::Clear, 80, 24);
        assert_eq!(sys.particles().len(), 0);
    }

    #[test]
    fn storm_has_more_particles_than_drizzle() {
        let storm = ParticleSystem::new(Condition::Thunderstorm, 80, 24);
        let drizzle = ParticleSystem::new(Condition::Drizzle, 80, 24);
        assert!(storm.particles().len() > drizzle.particles().len());
    }

    #[test]
    fn ticking_moves_rain_downward() {
        let mut sys = ParticleSystem::new(Condition::Rain, 80, 24);
        let before: Vec<f64> = sys.particles().iter().map(|p| p.y).collect();
        sys.tick(0.1);
        let after: Vec<f64> = sys.particles().iter().map(|p| p.y).collect();
        assert_eq!(before.len(), after.len());
        // At least some particles should have moved down (allowing for respawns resetting to -1).
        let moved_down = before
            .iter()
            .zip(after.iter())
            .filter(|(b, a)| **a > **b)
            .count();
        assert!(moved_down > 0);
    }

    #[test]
    fn resize_updates_bounds_and_repopulates() {
        let mut sys = ParticleSystem::new(Condition::Snow, 40, 10);
        let small_count = sys.particles().len();
        sys.resize(400, 100);
        assert!(sys.particles().len() >= small_count);
    }
}
