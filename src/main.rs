use std::fs::File;
use std::io::{BufWriter, Read, Write};

const RULE_NUMBER: u32 = 25_165_440;
const RING_SIZE: usize = 1024;
const RING_RANDOM_BIT_NUMBER: usize = RING_SIZE >> 1;
const OUTPUT_FILE_SIZE_BYTES: usize = 1 << 30;

fn main() {
    if std::env::args().count() < 3 {
        eprintln!("Usage: {} <random_seed_file_path> <output_file_path>", std::env::args().nth(0).unwrap());
        return;
    }
    let random_seed_file_path = std::env::args().nth(1).unwrap();
    let output_file_path = std::env::args().nth(2).unwrap();

    let mut ca_random_generator = CARandomGenerator::new(RULE_NUMBER, random_seed_file_path.as_str());

    let output_file = File::create(output_file_path).expect("Could not create file");
    let mut wr = BufWriter::new(&output_file);
    //output_file.set_len(OUTPUT_FILE_SIZE_BYTES as u64).expect("Could not set file length");
    for i in 0..OUTPUT_FILE_SIZE_BYTES {
        if i % 0x10000 == 0 {
            println!("{}k", i >> 10);
        }
        let byte = ca_random_generator.get_random_byte();
        wr.write(&[byte]).expect("Could not write to file");
    }
    wr.flush().unwrap();
}

struct CARandomGenerator {
    rings: [[bool; RING_SIZE]; 2], // 2 rings to avoid copy
    current_ring_index: usize,
    ring_modulo_mask: usize, // to avoid modulo operation
    rule_number: u32,
}

impl CARandomGenerator {
    pub fn new(rule_number: u32, seed_file_path: &str) -> Self {
        if RING_SIZE.count_ones() != 1 {
            panic!("Ring size must be a power of 2");
        }
        Self {
            rings: [Self::get_seeded_ring_from_bin_file(seed_file_path), [false; RING_SIZE]],
            current_ring_index: 0,
            ring_modulo_mask: (1 << RING_SIZE.trailing_zeros()) - 1,
            rule_number,
        }
    }

    fn get_seeded_ring_from_bin_file(file_path: &str) -> [bool; RING_SIZE] {
        let mut ring = [false; RING_SIZE];
        let mut file = File::open(file_path).expect("File not found");
        for i in 0..(RING_SIZE >> 3) {
            let mut buf = [0u8; 1];
            file.read(&mut buf).expect("Could not read file");
            for bit in 0..8 {
                ring[(i << 3) + bit] = (buf[0] & (1 << bit)) != 0;
            }
        }
        ring
    }

    fn update_ring(&mut self) {
        let [ring1, ring2] = &mut self.rings; // to avoid borrowing self twice
        let (current_ring, next_ring) = if self.current_ring_index == 0 {
            (ring1, ring2)
        } else {
            (ring2, ring1)
        };
        unsafe {
            for i in 0..RING_SIZE {
                let input_bits =
                    (*current_ring.get_unchecked((i + RING_SIZE - 2) & self.ring_modulo_mask) as u8) << 4
                        | (*current_ring.get_unchecked((i + RING_SIZE - 1) & self.ring_modulo_mask) as u8) << 3
                        | (*current_ring.get_unchecked(i) as u8) << 2
                        | (*current_ring.get_unchecked((i + 1) & self.ring_modulo_mask) as u8) << 1
                        | *current_ring.get_unchecked((i + 2) & self.ring_modulo_mask) as u8;
                *next_ring.get_unchecked_mut(i) = Self::compute_ca_rule(self.rule_number, input_bits);
            }
        }
        self.current_ring_index = 1 - self.current_ring_index; // swap rings
    }

    #[inline(always)]
    fn compute_ca_rule(rule_number: u32, input_bits: u8) -> bool {
        #[cfg(debug_assertions)]
        if input_bits > 31 {
            panic!("Input bits must be less than 32");
        }
        return (rule_number & (1 << input_bits)) != 0;
    }

    pub fn get_random_bit(&mut self) -> bool {
        self.update_ring();
        self.rings[self.current_ring_index][RING_RANDOM_BIT_NUMBER]
    }
    pub fn get_random_byte(&mut self) -> u8 {
        let mut byte = 0;
        for i in 0..8 {
            byte |= (self.get_random_bit() as u8) << i;
        }
        byte
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_get_random_bit() {
        let mut ca_random_generator = super::CARandomGenerator::new(25_165_440, "random_seed.bin");
        assert_eq!(ca_random_generator.get_random_bit(), false);
        assert_eq!(ca_random_generator.get_random_bit(), true);
    }

    #[test]
    fn test_get_random_bytes() {
        let mut ca_random_generator = super::CARandomGenerator::new(25_165_440, "random_seed.bin");
        assert_eq!(ca_random_generator.get_random_byte(), 222);
        assert_eq!(ca_random_generator.get_random_byte(), 121);
    }
}