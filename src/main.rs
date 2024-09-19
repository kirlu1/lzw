use std::collections::BTreeMap;

use std::fs::File;
use std::io::{Read, Write};

fn main() {

    let encoder = LzwEncoder::new();

    let mut buffer = String::new();
    File::open("folktale.txt")
        .expect("failed opening file")
        .read_to_string(&mut buffer)
        .expect("failed reading into buffer");

    buffer = buffer.to_lowercase();

    println!("{}", buffer);
    let uncompressed_size = buffer.len();
    
    let encoded = encoder.encode(buffer.as_bytes());
    let compressed_size = encoded.bits.len();

    encoded.to_file("compressed.txt");

    println!("Uncompressed size: {}", uncompressed_size);
    println!("Compressed size: {}", compressed_size);

    println!("Percent size of original: {}%", (compressed_size as f64 / uncompressed_size as f64) * 100f64)
}

type IntType = usize;

const INTTYPE_BITS : IntType = (0 as IntType).count_zeros() as IntType;

// Fortunately, all characters in our initial alphabet are defined by a single byte
const ALPHABET: &str = " abcdefghijklmnopqrstuvwxyzæøå";

#[derive(Debug, Clone)]
struct Bits {
    bits: Vec<IntType>,
    size: IntType,
}

struct LzwEncoder {
    dict: BTreeMap<Vec<u8>, Vec<IntType>>,
    word_size: IntType,
    next_word: Vec<IntType>,
}

struct LzwDecoder {
    bitstream: Vec<IntType>,
    dict: BTreeMap<Vec<IntType>, Vec<u8>>,
    word_size: IntType,

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

impl LzwEncoder {
    fn new() -> Self {
        let dict = BTreeMap::<Vec<u8>, Vec<IntType>>::new();

        let next_word = vec![0];

        let mut encoder = LzwEncoder { dict, word_size : 0, next_word };
        
        for c in ALPHABET.bytes() {
            encoder.insert(vec![c]);
        }

        encoder
    }

    /// Attempt to insert a new sequence of characters into the dictionary with a new codeword.
    /// 
    /// Returns false if the sequence already has a codeword.
    fn insert(&mut self, sequence : Vec<u8>) -> bool {
        if self.dict.contains_key(&sequence) {
            return false;
        }
        
        self.dict.insert(sequence, self.next_word.clone());

        self.increment_word();

        self.word_size = ((self.dict.len() - 1).checked_ilog2().unwrap_or(0) + 1) as IntType;

        return true
    }

    fn increment_word(&mut self) {
        let mut changed : bool;
        for n in self.next_word.iter_mut().rev() {
            *n+=1;
            if *n != 0 {
                break
            }
        }

        if self.next_word[0] == 0 {
            self.next_word.push(0);
            self.next_word[0]+=1;
        }
    }

    fn encode(mut self, input : &[u8]) -> Bits {

        let mut output : Bits = Bits { bits : vec![], size : 0 };
        let mut sequence: Vec<u8> = vec![];

        for i in 0..input.len() {
            sequence.push(input[i]);    

            if self.insert(sequence.clone()) {
                // Insert succeeds
                sequence.pop();

                if sequence.len() == 0 {
                    panic!("Alphabet not comprehensive");
                }
                let codeword = self.dict.get(&sequence).expect("encoding should exist").clone();
                sequence.clear();
                
                output = output.concat(Bits::new(codeword, self.word_size));
            }
        }

        if !sequence.is_empty() {
            let codeword = self.dict.get(&sequence).expect("encoding should exist 2").clone();
            output = output.concat(Bits::new(codeword, self.word_size));
        }

        output
    }
}


// cScSc
impl LzwDecoder {
    fn new() -> Self {
        todo!()
    }
}



