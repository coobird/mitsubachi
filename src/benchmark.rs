pub mod benchmark {
    use std::time::Instant;
    use sha2::Sha256;
    use sha2::Digest;

    pub fn benchmark() {
        let times = 5;
        let mut durations = Vec::new();
        for _ in 0 .. times {
            let data = vec![0u8; 1024 * 1024 * 25];
            let mut hasher = Sha256::new();
            let start_time = Instant::now();
            hasher.update(data);
            hasher.finalize();
            let duration = Instant::now().duration_since(start_time);
            durations.push(duration.as_micros());
        }
        println!("durations: {:?}", durations);
        // let option = durations.reduce(|accum: f64, item: u128| {
        //     accum + f64::from(item)
        // });
        let total: u128 = durations.iter().sum();
        let average_duration = total as f64 / times as f64;
        println!("average: {} us", average_duration);
    }
}