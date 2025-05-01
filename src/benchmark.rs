// Copyright (c) 2022-2025 Chris Kroells
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// 
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

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