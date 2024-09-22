mod lzw;
use lzw::*;

mod huffman;
use huffman::*;

use std::fs::File;
use std::io::{stdout, Read, Write};

type IntType = usize;

const INTTYPE_BITS : IntType = (0 as IntType).count_zeros() as IntType;

fn main() {
    let encoder = LzwEncoder::new();
    let mut buffer = String::new();
    File::open("folktale.txt").unwrap().read_to_string(&mut buffer).unwrap();

    buffer = buffer.to_lowercase();
    let input = buffer.chars()
    .collect::<Vec<char>>();

    let uncompressed_size = input.len();
    
    let lzw_encoded = encoder.encode(input);

    let bytes: &[u8] = bytemuck::cast_slice(&lzw_encoded.bits[..]);
    let compressed_size = bytes.len();
    
    lzw_encoded.clone().to_file("lzw_compressed.bin");

    println!("Uncompressed size: {}", uncompressed_size);
    println!("Compressed size: {}", compressed_size);
    println!("Percent size of original: {:.2}%", (compressed_size as f64 / uncompressed_size as f64) * 100f64);

    let decoder = LzwDecoder::new();
    let uncompressed = decoder.decode(lzw_encoded.bits.clone());
    File::create("lzw_uncompressed.txt").unwrap().write_all(uncompressed.as_bytes()).unwrap();


    // Further compress with Huffman encoding
    let hm_lzw_encoded = HuffmanEncoder::new().encode(&usize_to_u8(&lzw_encoded.bits));
    println!("\nHuffman compressed size (with/without tree): {} / {}", hm_lzw_encoded.len(), hm_lzw_encoded.len() - 512);
    println!("Percent of previous previous compressed size: {:.2}%", (hm_lzw_encoded.len() as f64 / compressed_size as f64) * 100f64);
    println!("Percent size of original: {:.2}%", (hm_lzw_encoded.len() as f64 / uncompressed_size as f64) * 100f64);

    File::create("hm_lzw_compressed.bin").unwrap().write_all(&mut hm_lzw_encoded.clone()).unwrap();

    let hm_decoded = HuffmanDecoder::new().decode(&hm_lzw_encoded);
    let bytes: Vec<usize> = u8_to_usize(hm_decoded);
    let hm_lzw_decoded = LzwDecoder::new().decode(bytes);

    File::create("hm_lzw_decoded.txt").unwrap().write(hm_lzw_decoded.as_bytes()).unwrap();

}


#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
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

    // Push a single bit onto a right adjusted Bits
    fn push_from_left(&mut self, value : IntType) {
        if self.size % INTTYPE_BITS == 0 {
            let mut front_bit = vec![0];
            std::mem::swap(&mut self.bits, &mut front_bit);
            self.bits.extend(front_bit);
        }

        let shift = self.size % INTTYPE_BITS;
        let mask = value << shift;
        self.bits[0] |= mask;

        self.size+=1;
    }
}


fn usize_to_u8(i : &[usize]) -> Vec<u8> {
    i.into_iter()
        .flat_map(
            |i|
                bytemuck::cast::<usize, [u8; 8]>(*i).into_iter().rev()
            )
        .collect::<Vec<u8>>()
}


fn u8_to_usize(mut i : Vec<u8>) -> Vec<usize> {
    let ratio = INTTYPE_BITS / 8; 
    let disalignment = (ratio - (i.len() % ratio)) % ratio;
    i.extend(vec![0; disalignment]);

    let (mut segment, mut remainder) = i.split_at(ratio);
    let mut new_vec : Vec<u8> = vec![];
    new_vec.extend(segment.iter().rev());

    while remainder.len() > 0 {
        (segment, remainder) = remainder.split_at(ratio);
        new_vec.extend(segment.iter().rev());
    }

    bytemuck::cast_slice::<u8, usize>(&new_vec[..]).to_vec()
}






mod bits_test {
    use crate::{Bits, IntType};

    #[test]
    fn concat_test() {
        let input1 = Bits {
            bits : vec![
                0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                size : 1,
            };
        let input2 = Bits { bits : vec![1], size : 1};

        let target = Bits {
            bits : vec![
                0b11000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                size : 2,
            };

        assert_eq!(input1.concat(input2), target);

        let input1 = Bits {
            bits : vec![
                0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
                ],
                size : 1,
            };
        let input2 = Bits { bits : vec![
            0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001,
            0b00000000_00000000_00000000_00000000_00000000_00000000_00000000_00000001,
        ], size : 65};

        let target = Bits {
            bits : vec![
                0b11000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                0b01000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000,
                ],
                size : 66,
            };

        assert_eq!(input1.concat(input2), target);

    }


    #[test]
    fn left_shift_test() {
        let input = Bits { bits : vec![1], size : 1};
        let target = Bits {
            bits : vec![
                0b10000000_00000000_00000000_00000000_00000000_00000000_00000000_00000000
            ],
            size : 1,
        };
    
        assert_eq!(input.shift_left(), target);

    }

}






#[cfg(target_arch="x86_64")]
mod casting_tests {
    use crate::*;


    #[test]
    fn u8_to_usize_test() {
        let inp : &[u8] = &[0b0000_0001, 0b0000_0000, 0b0000_0011, 0b0000_0000, 0b0000_0111, 0b0000_0000, 0b0000_1111, 0b0000_0000];
        let output = u8_to_usize(inp.to_vec());
        let target  = &[0b0000_0001_0000_0000_0000_0011_0000_0000_0000_0111_0000_0000_0000_1111_0000_0000usize];

        assert_eq!(*target, *output);
    }

    #[test]
    fn usize_to_u8_test() {
        let inp = [0b0000_0001_0000_0000_0000_0011_0000_0000_0000_0011_0000_0000_0000_1001_0000_0000usize];
        let output = usize_to_u8(&inp);
        let target : &[u8] = &[0b0000_0001, 0b0000_0000, 0b0000_0011, 0b0000_0000, 0b0000_0011, 0b0000_0000, 0b0000_1001, 0b0000_0000];

        assert_eq!(target, &output);
    }



}