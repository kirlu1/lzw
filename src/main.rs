mod lzw;
use lzw::*;

use std::fs::File;
use std::io::{Read, Write};

type IntType = usize;

const INTTYPE_BITS : IntType = (0 as IntType).count_zeros() as IntType;

fn main() {

    let encoder = LzwEncoder::new();

    let mut buffer = String::new();
    File::open("folktale.txt")
        .expect("failed opening file")
        .read_to_string(&mut buffer)
        .expect("failed reading into buffer");

    buffer = buffer.to_lowercase();
    let input = buffer.chars()
    .collect::<Vec<char>>();

    let uncompressed_size = input.len();
    
    let encoded = encoder.encode(input);
    let compressed_size = encoded.bits.len();

    let to_decompress = encoded.bits.clone();
    
    encoded.to_file("compressed.txt");

    println!("Uncompressed size: {}", uncompressed_size);
    println!("Compressed size: {}", compressed_size);

    println!("Percent size of original: {}%", (compressed_size as f64 / uncompressed_size as f64) * 100f64);

    let decoder = LzwDecoder::new();

    let uncompressed = decoder.decode(to_decompress);

    File::create("uncompressed.txt").unwrap().write_all(uncompressed.as_bytes()).expect("coulnd't write uncompressed");

}


#[derive(Debug, Clone)]
struct Bits {
    bits: Vec<IntType>,
    size: IntType,
}

impl Bits {
    fn new(val : Vec<IntType>, word_size : IntType) -> Self {
        let mut bits = val;

        let available_bits = bits.len() as IntType * INTTYPE_BITS;

        if word_size > available_bits {
            let pad_size = (word_size - available_bits) / INTTYPE_BITS;
            let mut padded = vec![0; pad_size as usize];

            padded.append(&mut bits);
            bits = padded;
        }

        Bits {
            bits,
            size : word_size,
        }
    }

    fn unused(&self) -> IntType {
        self.bits.len() as IntType * INTTYPE_BITS - self.size
    }

    /// Concatenates two Bits into one Bits, with their combined size. 
    /// 
    /// Assumes self is left-adjusted, and other is right-adjusted.
    fn concat(mut self, mut other : Bits) -> Bits {
        let combined_size = self.size + other.size;

        let remainder_shift = self.unused(); // Space left for other's bits
        let filler_shift = INTTYPE_BITS - remainder_shift;

        other = other.shift_left();

        if self.unused() == 0 {
            self.bits.append(&mut other.bits);
            return Bits { bits : self.bits, size : combined_size }
        }

        // Shuffle other's bits backwards
        for i in other.bits {
            let end = self.bits.last_mut().unwrap();
            let fill = i >> filler_shift;
            let remainder = i << remainder_shift;

            *end |= fill;
            self.bits.push(remainder);
        }

        let mut new = Bits { bits : self.bits, size : combined_size };

        while new.unused() >= INTTYPE_BITS {
            new.bits.pop();
        }
        new
    }

    /// Shifts from the right to the left
    fn shift_left(mut self) -> Bits {
        let difference = self.unused();
    
        let mut new_bits = vec![];
    
        self.bits.push(0);
        for w in self.bits.windows(2) {
            let mut a = w[0];
            let mut b = w[1];
    
            a <<= difference;
            b >>= INTTYPE_BITS - difference;
            new_bits.push(a | b);
        }
    
        Bits {
            bits : new_bits,
            size : self.size
        }
    }

    /// Creates a file or truncates an existing one of name `filename`,
    /// then writes the Bits object *as bits* to the file
    fn to_file(self, filename : &str) {
        let mut newfile = File::create(filename).expect("couldn't create file");

        let bytes = bytemuck::cast_slice::<IntType, u8>(&self.bits[..]);

        newfile.write_all(bytes).expect("failed to write entire buffer");
    }
}