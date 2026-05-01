use std::env;
use std::time::{Duration, Instant};

use wassan_engine::WassanMemoryEngine;
use wassan_holographic::{
    HoloNttCtx, WassanHologram, HOLOGRAM_CAPACITY, LANE_PRIMES, is_prime,
};

#[derive(Clone, Copy)]
struct Config {
    samples: usize,
    bank_size: usize,
}

#[derive(Clone, Copy)]
struct CaseResult {
    iterations: usize,
    total: Duration,
}

impl CaseResult {
    fn avg_micros(self) -> f64 {
        self.total.as_secs_f64() * 1_000_000.0 / self.iterations as f64
    }

    fn total_millis(self) -> f64 {
        self.total.as_secs_f64() * 1_000.0
    }
}

fn main() {
    let config = parse_args();
    let primes: Vec<u64> = LANE_PRIMES.iter().copied().filter(|p| is_prime(*p)).take(4).collect();
    let ctx = HoloNttCtx::new(&primes);

    println!("Wasan benchmark");
    println!("samples     : {}", config.samples);
    println!("bank_size   : {}", config.bank_size);
    println!("lanes       : {}", ctx.num_lanes());
    println!("capacity    : {} bytes", HOLOGRAM_CAPACITY);
    println!();
    println!("{:<26} {:>10} {:>14} {:>14}", "case", "iters", "total_ms", "avg_us");
    println!("{:-<68}", "");

    let small_payload = payload(96, 7);
    let half_payload = payload(HOLOGRAM_CAPACITY / 2, 19);
    let full_payload = payload(HOLOGRAM_CAPACITY, 31);

    report("encode_small", bench(config.samples, || {
        let _ = WassanHologram::encode(&small_payload, &ctx);
    }));

    report("encode_half", bench(config.samples, || {
        let _ = WassanHologram::encode(&half_payload, &ctx);
    }));

    report("encode_full", bench(config.samples, || {
        let _ = WassanHologram::encode(&full_payload, &ctx);
    }));

    let full_hologram = WassanHologram::encode(&full_payload, &ctx);
    report("decode_full", bench(config.samples, || {
        let _ = full_hologram.decode();
    }));

    report("decode_verified_full", bench(config.samples, || {
        let _ = full_hologram.decode_verified(&ctx).unwrap();
    }));

    report("syndrome_clean", bench(config.samples, || {
        let _ = full_hologram.compute_syndrome(&ctx);
    }));

    report("self_correct_3_sectors", bench(config.samples, || {
        let mut hologram = full_hologram.clone();
        hologram.corrupt_sector(10, 41, &primes);
        hologram.corrupt_sector(77, 97, &primes);
        hologram.corrupt_sector(201, 131, &primes);
        let _ = hologram.self_correct(&ctx);
    }));

    report("recover_from_sectors", bench(config.samples, || {
        let mut hologram = full_hologram.clone();
        hologram.corrupt_sector(32, 17, &primes);
        hologram.corrupt_sector(144, 29, &primes);
        let _ = hologram.recover_from_sectors_only(&ctx);
    }));

    let bank_store = bench(1, || {
        let mut engine = WassanMemoryEngine::new();
        engine.create_basin("bench", config.bank_size);
        for idx in 0..config.bank_size {
            let data = payload(128 + (idx % 32), idx as u8);
            let _ = engine.store("bench", &data).unwrap();
        }
    });
    report("engine_store_batch", bank_store);

    let mut engine = WassanMemoryEngine::new();
    engine.create_basin("bench", config.bank_size);
    let mut corpus = Vec::with_capacity(config.bank_size);
    for idx in 0..config.bank_size {
        let data = payload(128 + (idx % 32), idx as u8);
        let _ = engine.store("bench", &data).unwrap();
        corpus.push(data);
    }

    report("engine_retrieve_by_id", bench(config.samples, || {
        let target = ((config.bank_size / 2).max(1)) as u64;
        let _ = engine.retrieve_by_id("bench", target).unwrap();
    }));

    let query = corpus[(config.bank_size / 2).min(corpus.len().saturating_sub(1))].clone();
    report("engine_retrieve_by_content", bench(config.samples, || {
        let _ = engine.retrieve_by_content("bench", &query).unwrap();
    }));
}

fn parse_args() -> Config {
    let mut samples = 64usize;
    let mut bank_size = 64usize;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--samples" => {
                if let Some(value) = args.next() {
                    if let Ok(parsed) = value.parse::<usize>() {
                        samples = parsed.max(1);
                    }
                }
            }
            "--bank-size" => {
                if let Some(value) = args.next() {
                    if let Ok(parsed) = value.parse::<usize>() {
                        bank_size = parsed.max(1);
                    }
                }
            }
            _ => {}
        }
    }

    Config { samples, bank_size }
}

fn payload(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|idx| seed.wrapping_add((idx as u8).wrapping_mul(17)).rotate_left((idx % 7) as u32))
        .collect()
}

fn bench<F>(iterations: usize, mut f: F) -> CaseResult
where
    F: FnMut(),
{
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    CaseResult {
        iterations,
        total: start.elapsed(),
    }
}

fn report(name: &str, result: CaseResult) {
    println!(
        "{:<26} {:>10} {:>14.3} {:>14.3}",
        name,
        result.iterations,
        result.total_millis(),
        result.avg_micros()
    );
}
