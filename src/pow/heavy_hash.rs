use crate::{
    pow::{hasher::HeavyHasher, xoshiro::XoShiRo256PlusPlus},
    Hash,
};
use std::mem::MaybeUninit;


#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Clone)]
pub struct Matrix([[u16; 64]; 64]);

impl Matrix {
    // pub fn generate(hash: Hash) -> Self {
    //     let mut generator = XoShiRo256PlusPlus::new(hash);
    //     let mut mat = Matrix([[0u16; 64]; 64]);
    //     loop {
    //         for i in 0..64 {
    //             for j in (0..64).step_by(16) {
    //                 let val = generator.u64();
    //                 for shift in 0..16 {
    //                     mat.0[i][j + shift] = (val >> (4 * shift) & 0x0F) as u16;
    //                 }
    //             }
    //         }
    //         if mat.compute_rank() == 64 {
    //             return mat;
    //         }
    //     }
    // }

    #[inline(always)]
    pub fn generate(hash: Hash) -> Self {
        let mut generator = XoShiRo256PlusPlus::new(hash);
        loop {
            let mat = Self::rand_matrix_no_rank_check(&mut generator);
            if mat.compute_rank() == 64 {
                return mat;
            }
        }
    }

    #[inline(always)]
    fn rand_matrix_no_rank_check(generator: &mut XoShiRo256PlusPlus) -> Self {
        Self(array_from_fn(|_| {
            let mut val = 0;
            array_from_fn(|j| {
                let shift = j % 16;
                if shift == 0 {
                    val = generator.u64();
                }
                (val >> (4 * shift) & 0x0F) as u16
            })
        }))
    }

    #[inline(always)]
    fn convert_to_float(&self) -> [[f64; 64]; 64] {
        // SAFETY: An uninitialized MaybrUninit is always safe.
        let mut out: [[MaybeUninit<f64>; 64]; 64] = unsafe { MaybeUninit::uninit().assume_init() };

        out.iter_mut().zip(self.0.iter()).for_each(|(out_row, mat_row)| {
            out_row.iter_mut().zip(mat_row).for_each(|(out_element, &element)| {
                out_element.write(f64::from(element));
            });
        });
        // SAFETY: The loop above wrote into all indexes.
        unsafe { std::mem::transmute(out) }
    }

    pub fn compute_rank(&self) -> usize {
        const EPS: f64 = 1e-9;
        let mut mat_float = self.convert_to_float();
        let mut rank = 0;
        let mut row_selected = [false; 64];
        for i in 0..64 {
            if i >= 64 {
                // Required for optimization, See https://github.com/rust-lang/rust/issues/90794
                unreachable!()
            }
            let mut j = 0;
            while j < 64 {
                if !row_selected[j] && mat_float[j][i].abs() > EPS {
                    break;
                }
                j += 1;
            }
            if j != 64 {
                rank += 1;
                row_selected[j] = true;
                for p in (i + 1)..64 {
                    mat_float[j][p] /= mat_float[j][i];
                }
                for k in 0..64 {
                    if k != j && mat_float[k][i].abs() > EPS {
                        for p in (i + 1)..64 {
                            mat_float[k][p] -= mat_float[j][p] * mat_float[k][i];
                        }
                    }
                }
            }
        }
        rank
    }

    // ***Anti-FPGA Sidedoor***
    fn chaotic_random(x: u32) -> u32 {
        x.wrapping_mul(362605) ^ 0xA5A5A5A5
    }
    
    fn memory_intensive_mix(seed: u32) -> u32 {
        let mut acc = seed;
        for i in 0..32 {
            acc = acc.wrapping_mul(16625) ^ i;
        }
        acc
    }

    
    fn recursive_fibonacci_modulated(mut x: u32, depth: u8) -> u32 {
        let mut a = 1u32;
        let mut b = x | 1;
        
        let actual_depth = depth.min(8);
    
        for _ in 0..actual_depth {
            let temp = b;
            b = b.wrapping_add(a ^ (x.rotate_left((b % 17) as u32)));
            a = temp;
            x = x.rotate_right((a % 13) as u32) ^ b;
        }
    
        x
    }
    
    fn anti_fpga_hash(input: u32) -> u32 {
        let mut x = input;
        let noise = Self::memory_intensive_mix(x);
        let depth = ((noise & 0x0F) + 10) as u8;
    
        let prime_factor_sum = x.count_ones() as u32;
    
        x ^= prime_factor_sum;
    
        x = Self::recursive_fibonacci_modulated(x ^ noise, depth);
        x ^= Self::memory_intensive_mix(x.rotate_left(9));
    
        x
    }
    
    fn compute_after_comp_product(pre_comp_product: [u8; 32]) -> [u8; 32] {
        let mut after_comp_product = [0u8; 32];
    
        for i in 0..32 {
            let input = pre_comp_product[i] as u32 ^ ((i as u32) << 8);
            let normalized_input = input % 256;
            let modified_input = Self::chaotic_random(normalized_input);
    
            let hashed = Self::anti_fpga_hash(modified_input);
            after_comp_product[i] = (hashed & 0xFF) as u8;
        }
    
        after_comp_product
    }
    
    // ***Octionion Multiply***
    fn octonion_multiply(a: &[i64; 8], b: &[i64; 8]) -> [i64; 8] {
        let mut result = [0; 8];

         /*
            Multiplication table of octonions (non-commutative):

                ×    |  1   e₁   e₂   e₃   e₄   e₅   e₆   e₇  
                ------------------------------------------------
                1    |  1   e₁   e₂   e₃   e₄   e₅   e₆   e₇  
                e₁   | e₁  -1   e₃  -e₂   e₅  -e₆   e₄  -e₇  
                e₂   | e₂  -e₃  -1    e₁   e₆   e₄  -e₅   e₇  
                e₃   | e₃   e₂  -e₁  -1    e₄  -e₇   e₆  -e₅  
                e₄   | e₄  -e₅  -e₆  -e₄  -1    e₇   e₂   e₃  
                e₅   | e₅   e₆   e₄   e₇  -e₇  -1   -e₃   e₂  
                e₆   | e₆  -e₄  -e₅   e₆  -e₂   e₃  -1    e₁  
                e₇   | e₇   e₄  -e₇   e₅  -e₃  -e₂   e₁  -1  
        */
        
         // e0
        result[0] = a[0].wrapping_mul(b[0])
            .wrapping_sub(a[1].wrapping_mul(b[1]))
            .wrapping_sub(a[2].wrapping_mul(b[2]))
            .wrapping_sub(a[3].wrapping_mul(b[3]))
            .wrapping_sub(a[4].wrapping_mul(b[4]))
            .wrapping_sub(a[5].wrapping_mul(b[5]))
            .wrapping_sub(a[6].wrapping_mul(b[6]))
            .wrapping_sub(a[7].wrapping_mul(b[7]));
        
         // e1
        result[1] = a[0].wrapping_mul(b[1])
            .wrapping_add(a[1].wrapping_mul(b[0]))
            .wrapping_add(a[2].wrapping_mul(b[3]))
            .wrapping_sub(a[3].wrapping_mul(b[2]))
            .wrapping_add(a[4].wrapping_mul(b[5]))
            .wrapping_sub(a[5].wrapping_mul(b[4]))
            .wrapping_sub(a[6].wrapping_mul(b[7]))
            .wrapping_add(a[7].wrapping_mul(b[6]));

         // e2
        result[2] = a[0].wrapping_mul(b[2])
            .wrapping_sub(a[1].wrapping_mul(b[3]))
            .wrapping_add(a[2].wrapping_mul(b[0]))
            .wrapping_add(a[3].wrapping_mul(b[1]))
            .wrapping_add(a[4].wrapping_mul(b[6]))
            .wrapping_sub(a[5].wrapping_mul(b[7]))
            .wrapping_add(a[6].wrapping_mul(b[4]))
            .wrapping_sub(a[7].wrapping_mul(b[5]));

       // e3
        result[3] = a[0].wrapping_mul(b[3])
            .wrapping_add(a[1].wrapping_mul(b[2]))
            .wrapping_sub(a[2].wrapping_mul(b[1]))
            .wrapping_add(a[3].wrapping_mul(b[0]))
            .wrapping_add(a[4].wrapping_mul(b[7]))
            .wrapping_add(a[5].wrapping_mul(b[6]))
            .wrapping_sub(a[6].wrapping_mul(b[5]))
            .wrapping_add(a[7].wrapping_mul(b[4]));
    
         // e4
        result[4] = a[0].wrapping_mul(b[4])
            .wrapping_sub(a[1].wrapping_mul(b[5]))
            .wrapping_sub(a[2].wrapping_mul(b[6]))
            .wrapping_sub(a[3].wrapping_mul(b[7]))
            .wrapping_add(a[4].wrapping_mul(b[0]))
            .wrapping_add(a[5].wrapping_mul(b[1]))
            .wrapping_add(a[6].wrapping_mul(b[2]))
            .wrapping_add(a[7].wrapping_mul(b[3]));
    
         // e5
        result[5] = a[0].wrapping_mul(b[5])
            .wrapping_add(a[1].wrapping_mul(b[4]))
            .wrapping_sub(a[2].wrapping_mul(b[7]))
            .wrapping_add(a[3].wrapping_mul(b[6]))
            .wrapping_sub(a[4].wrapping_mul(b[1]))
            .wrapping_add(a[5].wrapping_mul(b[0]))
            .wrapping_add(a[6].wrapping_mul(b[3]))
            .wrapping_add(a[7].wrapping_mul(b[2]));
    
         // e6
        result[6] = a[0].wrapping_mul(b[6])
            .wrapping_add(a[1].wrapping_mul(b[7]))
            .wrapping_add(a[2].wrapping_mul(b[4]))
            .wrapping_sub(a[3].wrapping_mul(b[5]))
            .wrapping_sub(a[4].wrapping_mul(b[2]))
            .wrapping_add(a[5].wrapping_mul(b[3]))
            .wrapping_add(a[6].wrapping_mul(b[0]))
            .wrapping_add(a[7].wrapping_mul(b[1]));

         // e7
        result[7] = a[0].wrapping_mul(b[7])
            .wrapping_sub(a[1].wrapping_mul(b[6]))
            .wrapping_add(a[2].wrapping_mul(b[5]))
            .wrapping_add(a[3].wrapping_mul(b[4]))
            .wrapping_sub(a[4].wrapping_mul(b[3]))
            .wrapping_add(a[5].wrapping_mul(b[2]))
            .wrapping_add(a[6].wrapping_mul(b[1]))
            .wrapping_add(a[7].wrapping_mul(b[0]));
        
        // Result
        return result;
    }

    // Octonion Hash
    fn octonion_hash(input_hash: &[u8; 32]) -> [i64; 8] {

        // Initialize the octonion with the first 8 bytes of the input_hash
        let mut oct = [
            input_hash[0] as i64,  // e0
            input_hash[1] as i64,  // e1
            input_hash[2] as i64,  // e2
            input_hash[3] as i64,  // e3
            input_hash[4] as i64,  // e4
            input_hash[5] as i64,  // e5
            input_hash[6] as i64,  // e6
            input_hash[7] as i64,  // e7
        ];

        // Loop through the remaining bytes of the input_hash        
        for i in 8..input_hash.len() {
            let rotation = [
                input_hash[i % 32] as i64,        // e0
                input_hash[(i + 1) % 32] as i64,  // e1
                input_hash[(i + 2) % 32] as i64,  // e2
                input_hash[(i + 3) % 32] as i64,  // e3
                input_hash[(i + 4) % 32] as i64,  // e4
                input_hash[(i + 5) % 32] as i64,  // e5
                input_hash[(i + 6) % 32] as i64,  // e6
                input_hash[(i + 7) % 32] as i64,  // e7
            ];

             // Perform octonion multiplication with the current rotation
            oct = Self::octonion_multiply(&oct, &rotation);
        }
    
        // Return the resulting octonion 
        oct
    }    

    pub fn heavy_hash(&self, hash: Hash) -> Hash {
        // Convert the hash to its byte representation
        let hash_bytes = hash.to_le_bytes();
    
        // Create an array containing the nibbles
        let mut nibbles = [0u8; 64];
        for (i, &byte) in hash_bytes.iter().enumerate() {
            nibbles[2 * i] = byte >> 4;
            nibbles[2 * i + 1] = byte & 0x0F;
        }
    
        // Matrix and vector multiplication
        let mut product = [0u8; 32];
        let mut nibble_product = [0u8; 32];

        for i in 0..32 {
            let mut sum1: u32 = 0;
            let mut sum2: u32 = 0;
            let mut sum3: u32 = 0;
            let mut sum4: u32 = 0;
    
            for j in 0..64 {
                let elem = nibbles[j] as u32;
                sum1 += (self.0[2 * i][j] as u32) * elem;
                sum2 += (self.0[2 * i + 1][j] as u32) * elem;
                sum3 += (self.0[1 * i + 2][j] as u32) * elem;
                sum4 += (self.0[1 * i + 3][j] as u32) * elem;                
            }

           // Nibbles
           //A
           let a_nibble = (sum1 & 0xF) ^ ((sum2 >> 4) & 0xF) ^ ((sum3 >> 8) & 0xF) 
                ^ ((sum1.wrapping_mul(0xABCD) >> 12) & 0xF) 
                ^ ((sum1.wrapping_mul(0x1234) >> 8) & 0xF)
                ^ ((sum2.wrapping_mul(0x5678) >> 16) & 0xF)
                ^ ((sum3.wrapping_mul(0x9ABC) >> 4) & 0xF)
                ^ ((sum1.rotate_left(3) & 0xF) ^ (sum3.rotate_right(5) & 0xF));  

            // B
            let b_nibble = (sum2 & 0xF) ^ ((sum1 >> 4) & 0xF) ^ ((sum4 >> 8) & 0xF) 
                ^ ((sum2.wrapping_mul(0xDCBA) >> 14) & 0xF)
                ^ ((sum2.wrapping_mul(0x8765) >> 10) & 0xF) 
                ^ ((sum1.wrapping_mul(0x4321) >> 6) & 0xF)
                ^ ((sum4.rotate_left(2) ^ sum1.rotate_right(1)) & 0xF); 

            // C
            let c_nibble = (sum3 & 0xF) ^ ((sum2 >> 4) & 0xF) ^ ((sum2 >> 8) & 0xF) 
                ^ ((sum3.wrapping_mul(0xF135) >> 10) & 0xF)
                ^ ((sum3.wrapping_mul(0x2468) >> 12) & 0xF) 
                ^ ((sum4.wrapping_mul(0xACEF) >> 8) & 0xF)
                ^ ((sum2.wrapping_mul(0x1357) >> 4) & 0xF)
                ^ ((sum3.rotate_left(5) & 0xF) ^ (sum1.rotate_right(7) & 0xF));

            // D
            let d_nibble = (sum1 & 0xF) ^ ((sum4 >> 4) & 0xF) ^ ((sum1 >> 8) & 0xF)
                ^ ((sum4.wrapping_mul(0x57A3) >> 6) & 0xF)
                ^ ((sum3.wrapping_mul(0xD4E3) >> 12) & 0xF)
                ^ ((sum1.wrapping_mul(0x9F8B) >> 10) & 0xF)
                ^ ((sum4.rotate_left(4) ^ sum1.wrapping_add(sum2)) & 0xF);

            // Combine c_nibble and d_nibble to form nibble_product
            nibble_product[i] = ((c_nibble << 4) | d_nibble) as u8; 
            
            // Combine a_nibble and b_nibble to form product
            product[i] = ((a_nibble << 4) | b_nibble) as u8;
        }
    
        // XOR the product with the original hash   
        product.iter_mut().zip(hash_bytes.iter()).for_each(|(p, h)| *p ^= h);
        nibble_product.iter_mut().zip(hash_bytes.iter()).for_each(|(p, h)| *p ^= h);

        let product_before_oct = product.clone();

        // ** Octonion Function **
        let octonion_result = Self::octonion_hash(&product);
        
        // XOR with i64 values - convert to u8
        for i in 0..32 {
            let oct_value = octonion_result[i / 8];
            
            // Extract the relevant byte from the i64 value
            let oct_value_u8 = ((oct_value >> (8 * (i % 8))) & 0xFF) as u8; 

            // XOR the values and store the result in the product
            product[i] ^= oct_value_u8;
        }

        // Debug before Sbox
        // println!("Product before calculation: {:?}", product);

        
        // **Nonlinear S-Box**
        let mut sbox: [u8; 256] = [0; 256];

        for i in 0..256 {
            let i = i as u8;
        
            let (source_array, rotate_left_val, rotate_right_val) = 
                if i < 16 { (&product, (nibble_product[3] ^ 0x4F).wrapping_mul(3) as u8, (hash_bytes[2] ^ 0xD3).wrapping_mul(5) as u8) }
                else if i < 32 { (&hash_bytes, (product[7] ^ 0xA6).wrapping_mul(2) as u8, (nibble_product[5] ^ 0x5B).wrapping_mul(7) as u8) }
                else if i < 48 { (&nibble_product, (product_before_oct[1] ^ 0x9C).wrapping_mul(9) as u8, (product[0] ^ 0x8E).wrapping_mul(3) as u8) }
                else if i < 64 { (&hash_bytes, (product[6] ^ 0x71).wrapping_mul(4) as u8, (product_before_oct[3] ^ 0x2F).wrapping_mul(5) as u8) }
                else if i < 80 { (&product_before_oct, (nibble_product[4] ^ 0xB2).wrapping_mul(3) as u8, (hash_bytes[7] ^ 0x6D).wrapping_mul(7) as u8) }
                else if i < 96 { (&hash_bytes, (product[0] ^ 0x58).wrapping_mul(6) as u8, (nibble_product[1] ^ 0xEE).wrapping_mul(9) as u8) }
                else if i < 112 { (&product, (product_before_oct[2] ^ 0x37).wrapping_mul(2) as u8, (hash_bytes[6] ^ 0x44).wrapping_mul(6) as u8) }
                else if i < 128 { (&hash_bytes, (product[5] ^ 0x1A).wrapping_mul(5) as u8, (hash_bytes[4] ^ 0x7C).wrapping_mul(8) as u8) }
                else if i < 144 { (&product_before_oct, (nibble_product[3] ^ 0x93).wrapping_mul(7) as u8, (product[2] ^ 0xAF).wrapping_mul(3) as u8) }
                else if i < 160 { (&hash_bytes, (product[7] ^ 0x29).wrapping_mul(9) as u8, (nibble_product[5] ^ 0xDC).wrapping_mul(2) as u8) }
                else if i < 176 { (&nibble_product, (product_before_oct[1] ^ 0x4E).wrapping_mul(4) as u8, (hash_bytes[0] ^ 0x8B).wrapping_mul(3) as u8) }
                else if i < 192 { (&hash_bytes, (nibble_product[6] ^ 0xF3).wrapping_mul(5) as u8, (product_before_oct[3] ^ 0x62).wrapping_mul(8) as u8) }
                else if i < 208 { (&product_before_oct, (product[4] ^ 0xB7).wrapping_mul(6) as u8, (product[7] ^ 0x15).wrapping_mul(2) as u8) }
                else if i < 224 { (&hash_bytes, (product[0] ^ 0x2D).wrapping_mul(8) as u8, (product_before_oct[1] ^ 0xC8).wrapping_mul(7) as u8) }
                else if i < 240 { (&product, (product_before_oct[2] ^ 0x6F).wrapping_mul(3) as u8, (nibble_product[6] ^ 0x99).wrapping_mul(9) as u8) }
                else { (&hash_bytes, (nibble_product[5] ^ 0xE1).wrapping_mul(7) as u8, (hash_bytes[4] ^ 0x3B).wrapping_mul(5) as u8) };      
        
            let value = 
                if i < 16 { (product[i as usize % 32].wrapping_mul(0x03).wrapping_add(i.wrapping_mul(0xAA))) & 0xFF }
                else if i < 32 { (hash_bytes[(i - 16) as usize % 32].wrapping_mul(0x05).wrapping_add((i - 16).wrapping_mul(0xBB))) & 0xFF }
                else if i < 48 { (product_before_oct[(i - 32) as usize % 32].wrapping_mul(0x07).wrapping_add((i - 32).wrapping_mul(0xCC))) & 0xFF }
                else if i < 64 { (nibble_product[(i - 48) as usize % 32].wrapping_mul(0x0F).wrapping_add((i - 48).wrapping_mul(0xDD))) & 0xFF }
                else if i < 80 { (product[(i - 64) as usize % 32].wrapping_mul(0x11).wrapping_add((i - 64).wrapping_mul(0xEE))) & 0xFF }
                else if i < 96 { (hash_bytes[(i - 80) as usize % 32].wrapping_mul(0x13).wrapping_add((i - 80).wrapping_mul(0xFF))) & 0xFF }
                else if i < 112 { (product_before_oct[(i - 96) as usize % 32].wrapping_mul(0x17).wrapping_add((i - 96).wrapping_mul(0x11))) & 0xFF }
                else if i < 128 { (nibble_product[(i - 112) as usize % 32].wrapping_mul(0x19).wrapping_add((i - 112).wrapping_mul(0x22))) & 0xFF }
                else if i < 144 { (product[(i - 128) as usize % 32].wrapping_mul(0x1D).wrapping_add((i - 128).wrapping_mul(0x33))) & 0xFF }
                else if i < 160 { (hash_bytes[(i - 144) as usize % 32].wrapping_mul(0x1F).wrapping_add((i - 144).wrapping_mul(0x44))) & 0xFF }
                else if i < 176 { (product_before_oct[(i - 160) as usize % 32].wrapping_mul(0x23).wrapping_add((i - 160).wrapping_mul(0x55))) & 0xFF }
                else if i < 192 { (nibble_product[(i - 176) as usize % 32].wrapping_mul(0x29).wrapping_add((i - 176).wrapping_mul(0x66))) & 0xFF }
                else if i < 208 { (product[(i - 192) as usize % 32].wrapping_mul(0x2F).wrapping_add((i - 192).wrapping_mul(0x77))) & 0xFF }
                else if i < 224 { (hash_bytes[(i - 208) as usize % 32].wrapping_mul(0x31).wrapping_add((i - 208).wrapping_mul(0x88))) & 0xFF }
                else if i < 240 { (product_before_oct[(i - 224) as usize % 32].wrapping_mul(0x37).wrapping_add((i - 224).wrapping_mul(0x99))) & 0xFF }
                else { (nibble_product[(i - 240) as usize % 32].wrapping_mul(0x3F).wrapping_add((i - 240).wrapping_mul(0xAA))) & 0xFF };           
        
            let rotate_left_shift = (product[(i as usize + 1) % product.len()] as u32 + i as u32) % 8;
            let rotate_right_shift = (hash_bytes[(i as usize + 2) % hash_bytes.len()] as u32 + i as u32) % 8;
        
            let rotation_left = rotate_left_val.rotate_left(rotate_left_shift);
            let rotation_right = rotate_right_val.rotate_right(rotate_right_shift);
        
            let index = (i as usize + rotation_left as usize + rotation_right as usize) % source_array.len();
            sbox[i as usize] = source_array[index] ^ value;
        }

        // Update Sbox Values
        let index = ((product_before_oct[2] % 8) + 1) as usize;  
        let iterations = 1 + (product[index] % 2);

        for _ in 0..iterations {
            let mut temp_sbox = sbox;

            for i in 0..256 {
                let mut value = temp_sbox[i];

                let rotate_left_shift = (product[(i + 1) % product.len()] as u32 + i as u32 + (i * 3) as u32) % 8;  
                let rotate_right_shift = (hash_bytes[(i + 2) % hash_bytes.len()] as u32 + i as u32 + (i * 5) as u32) % 8; 

                let rotated_value = value.rotate_left(rotate_left_shift) | value.rotate_right(rotate_right_shift);

                let xor_value = {
                    let base_value = (i as u8).wrapping_add(product[(i * 3) % product.len()] ^ hash_bytes[(i * 7) % hash_bytes.len()]) ^ 0xA5;
                    let shifted_value = base_value.rotate_left((i % 8) as u32); 
                    shifted_value ^ 0x55 
                };

                value ^= rotated_value ^ xor_value;
                temp_sbox[i] = value; 
            }

            sbox = temp_sbox;
        }

        // Anti FPGA Sidedoor
        let pre_comp_product: [u8; 32] = product;
        let after_comp_product = Self::compute_after_comp_product(pre_comp_product);
        
        // Blake3 Chaining
        let index_blake = ((product_before_oct[5] % 8) + 1) as usize;  
        let iterations_blake = 1 + (product[index_blake] % 3);

        let mut b3_hash_array = product.clone(); 
        for _ in 0..iterations_blake {
            // BLAKE3 Hashing
            let mut b3_hasher = blake3::Hasher::new();
            b3_hasher.update(&b3_hash_array);
            let product_blake3 = b3_hasher.finalize();
            let b3_hash_bytes = product_blake3.as_bytes();

            // Convert
            b3_hash_array.copy_from_slice(b3_hash_bytes);
        }

        // Sinus (Testnet)
        // let sinus_in = product.clone();    
        // let sinus_out = Self::sinusoidal_multiply(&sinus_in);

        // Apply S-Box to the product with XOR
        for i in 0..32 {
            let ref_array = match (i * 31) % 4 { 
                0 => &nibble_product,
                1 => &hash_bytes,
                2 => &product,
                _ => &product_before_oct,
            };

            let byte_val = ref_array[(i * 13) % ref_array.len()] as usize;

            let index = (byte_val 
                        + product[(i * 31) % product.len()] as usize 
                        + hash_bytes[(i * 19) % hash_bytes.len()] as usize 
                        + i * 41) % 256;  
            
           b3_hash_array[i] ^= sbox[index]; 
        }

        // Final Xor
        for i in 0..32 {
            b3_hash_array[i] ^= after_comp_product[i];
        }

        // println!("hash after: {:?}", b3_hash_array);

        // Return the calculated hash
        HeavyHasher::hash(Hash::from_le_bytes(b3_hash_array))
    }
}

pub fn array_from_fn<F, T, const N: usize>(mut cb: F) -> [T; N]
where
    F: FnMut(usize) -> T,
{
    let mut idx = 0;
    [(); N].map(|_| {
        let res = cb(idx);
        idx += 1;
        res
    })
}

#[cfg(test)]
mod tests {
    use crate::pow::heavy_hash::Matrix;
    use crate::pow::xoshiro::XoShiRo256PlusPlus;
    use crate::Hash;

    #[test]
    fn test_compute_rank() {
        let zero = Matrix([[0; 64]; 64]);
        assert_eq!(zero.compute_rank(), 0);
        let mut matrix = zero;
        let mut gen = XoShiRo256PlusPlus::new(Hash::from_le_bytes([42; 32]));
        matrix.0.iter_mut().for_each(|row| {
            row.iter_mut().for_each(|val| {
                *val = gen.u64() as u16;
            })
        });
        assert_eq!(matrix.compute_rank(), 64);

        matrix.0[0] = matrix.0[1];
        assert_eq!(matrix.compute_rank(), 63);
    }

    #[test]
    fn test_heavy_hash() {
        let expected_hash = Hash::from_le_bytes([
            135, 104, 159, 55, 153, 67, 234, 249, 183, 71, 92, 169, 83, 37, 104, 119, 114, 191, 204, 104, 252, 120,
            153, 202, 235, 68, 9, 236, 69, 144, 195, 37,
        ]);
        #[rustfmt::skip]
        let test_matrix = Matrix([
            [13, 2, 14, 13, 2, 15, 14, 3, 10, 4, 1, 8, 4, 3, 8, 15, 15, 15, 15, 15, 2, 11, 15, 15, 15, 1, 7, 12, 12, 4, 2, 0, 6, 1, 14, 10, 12, 14, 15, 8, 10, 12, 0, 5, 13, 3, 14, 10, 10, 6, 12, 11, 11, 7, 6, 6, 10, 2, 2, 4, 11, 12, 0, 5],
            [4, 13, 0, 2, 1, 15, 13, 13, 11, 2, 5, 12, 15, 7, 0, 10, 7, 2, 6, 3, 12, 0, 12, 0, 2, 6, 7, 7, 7, 7, 10, 12, 11, 14, 12, 12, 4, 11, 10, 0, 10, 11, 2, 10, 1, 7, 7, 12, 15, 9, 5, 14, 9, 12, 3, 0, 12, 13, 4, 13, 8, 15, 11, 6],
            [14, 6, 15, 9, 8, 2, 2, 12, 2, 3, 4, 12, 13, 15, 4, 5, 13, 4, 3, 0, 14, 3, 5, 14, 3, 13, 4, 15, 9, 12, 7, 15, 5, 1, 13, 12, 9, 9, 8, 11, 14, 11, 4, 10, 12, 6, 12, 8, 6, 3, 9, 8, 1, 6, 0, 5, 8, 9, 12, 5, 14, 15, 2, 2],
            [9, 6, 7, 6, 0, 11, 5, 6, 2, 14, 12, 6, 4, 13, 8, 9, 2, 1, 9, 7, 4, 5, 10, 8, 11, 11, 11, 15, 7, 11, 1, 14, 3, 8, 14, 8, 2, 8, 13, 7, 8, 8, 15, 7, 1, 13, 7, 9, 1, 7, 15, 15, 0, 0, 12, 15, 13, 5, 13, 10, 1, 5, 6, 13],
            [4, 0, 12, 10, 6, 11, 14, 2, 2, 15, 4, 1, 2, 4, 2, 12, 13, 1, 9, 10, 8, 0, 2, 10, 13, 8, 9, 7, 5, 3, 8, 2, 6, 6, 1, 12, 3, 0, 1, 4, 2, 8, 3, 13, 6, 15, 0, 13, 14, 4, 15, 0, 7, 3, 7, 8, 5, 14, 14, 5, 5, 0, 1, 2],
            [12, 14, 6, 3, 3, 4, 6, 7, 1, 3, 2, 7, 15, 15, 15, 10, 9, 12, 0, 6, 3, 8, 5, 0, 13, 5, 0, 6, 0, 14, 2, 12, 10, 4, 11, 2, 10, 7, 7, 6, 8, 11, 4, 4, 11, 9, 3, 12, 10, 5, 2, 6, 5, 5, 10, 13, 12, 10, 1, 6, 14, 7, 12, 4],
            [7, 14, 6, 7, 7, 12, 4, 1, 8, 6, 8, 13, 13, 5, 12, 14, 10, 8, 6, 2, 12, 3, 8, 15, 5, 15, 15, 3, 14, 0, 8, 6, 9, 12, 9, 7, 3, 8, 4, 0, 7, 14, 3, 3, 13, 14, 3, 7, 3, 2, 2, 3, 3, 12, 6, 7, 4, 1, 14, 10, 6, 10, 2, 9],
            [14, 11, 15, 5, 7, 10, 1, 11, 4, 2, 6, 2, 9, 7, 4, 0, 9, 12, 11, 2, 3, 13, 1, 5, 4, 10, 5, 6, 6, 12, 8, 1, 1, 15, 4, 2, 12, 12, 0, 4, 14, 3, 11, 1, 7, 5, 9, 4, 3, 15, 7, 3, 15, 9, 8, 3, 8, 3, 3, 6, 7, 6, 9, 2],
            [10, 4, 6, 10, 5, 2, 15, 12, 0, 14, 14, 15, 14, 0, 12, 9, 1, 12, 4, 5, 5, 2, 10, 4, 2, 13, 11, 3, 1, 8, 10, 0, 7, 0, 12, 4, 11, 1, 14, 6, 14, 5, 5, 11, 11, 1, 3, 8, 0, 6, 11, 11, 8, 4, 7, 6, 14, 4, 9, 14, 9, 7, 13, 9],
            [12, 7, 9, 8, 2, 3, 3, 5, 14, 8, 0, 9, 7, 4, 2, 15, 15, 3, 11, 11, 8, 5, 7, 5, 0, 15, 10, 8, 0, 13, 1, 14, 8, 10, 1, 4, 13, 1, 13, 3, 11, 11, 2, 3, 10, 6, 8, 14, 15, 2, 10, 10, 12, 7, 7, 6, 6, 3, 13, 8, 1, 14, 2, 1],
            [2, 11, 6, 9, 13, 3, 12, 6, 0, 4, 6, 13, 8, 14, 6, 9, 10, 2, 10, 8, 4, 13, 6, 5, 0, 13, 15, 4, 2, 2, 1, 7, 5, 3, 3, 13, 7, 3, 5, 9, 15, 14, 14, 6, 0, 15, 11, 2, 4, 15, 6, 9, 8, 9, 15, 2, 6, 9, 15, 8, 4, 4, 11, 1],
            [10, 11, 8, 3, 11, 13, 10, 2, 2, 5, 2, 14, 15, 10, 2, 11, 0, 1, 8, 2, 14, 1, 10, 0, 3, 7, 5, 10, 7, 8, 15, 7, 2, 5, 13, 4, 10, 3, 6, 2, 3, 9, 6, 11, 7, 14, 1, 11, 9, 3, 3, 7, 6, 0, 9, 11, 4, 10, 4, 1, 9, 7, 4, 15],
            [13, 8, 15, 14, 11, 12, 5, 3, 9, 14, 1, 5, 14, 13, 14, 5, 13, 5, 4, 10, 9, 9, 0, 0, 6, 12, 5, 7, 2, 7, 2, 6, 6, 6, 1, 12, 9, 15, 7, 11, 11, 10, 11, 1, 10, 10, 0, 8, 1, 4, 5, 5, 8, 10, 10, 15, 6, 8, 13, 11, 11, 3, 15, 5],
            [8, 11, 5, 10, 1, 10, 9, 1, 12, 7, 6, 11, 1, 1, 4, 1, 2, 8, 4, 4, 7, 7, 8, 2, 7, 1, 14, 1, 8, 15, 15, 12, 10, 4, 15, 11, 3, 6, 10, 7, 4, 0, 10, 9, 11, 7, 1, 14, 4, 14, 3, 14, 10, 4, 13, 12, 5, 3, 12, 7, 10, 8, 0, 3],
            [9, 11, 6, 15, 14, 10, 0, 4, 7, 7, 6, 0, 7, 7, 12, 15, 5, 4, 12, 3, 7, 3, 0, 12, 2, 7, 11, 6, 7, 3, 2, 8, 5, 11, 9, 4, 3, 8, 11, 12, 3, 5, 14, 12, 4, 13, 12, 0, 3, 14, 4, 9, 1, 1, 9, 14, 10, 14, 8, 15, 6, 14, 10, 15],
            [10, 14, 10, 0, 10, 12, 15, 0, 3, 9, 11, 10, 3, 5, 1, 1, 9, 1, 7, 15, 7, 8, 10, 10, 12, 11, 5, 1, 10, 3, 6, 6, 13, 0, 13, 1, 4, 5, 9, 4, 9, 15, 8, 4, 13, 13, 4, 5, 5, 11, 1, 13, 15, 3, 10, 15, 7, 11, 10, 15, 8, 12, 10, 3],
            [8, 5, 11, 3, 8, 13, 15, 15, 3, 12, 1, 13, 1, 7, 1, 5, 6, 13, 7, 8, 5, 1, 12, 3, 10, 7, 12, 6, 14, 12, 15, 5, 3, 12, 2, 15, 11, 13, 1, 13, 8, 5, 8, 0, 13, 15, 7, 13, 6, 13, 10, 1, 11, 0, 8, 9, 5, 11, 2, 9, 9, 10, 4, 15],
            [0, 4, 12, 14, 3, 1, 7, 5, 11, 13, 5, 3, 11, 12, 6, 8, 10, 15, 11, 8, 7, 10, 0, 2, 5, 15, 6, 10, 4, 2, 3, 1, 13, 7, 6, 12, 14, 7, 6, 14, 12, 10, 6, 14, 12, 0, 12, 11, 6, 9, 3, 1, 12, 15, 15, 3, 5, 5, 10, 11, 7, 15, 13, 3],
            [12, 14, 2, 14, 13, 6, 15, 7, 8, 8, 14, 13, 9, 2, 2, 10, 3, 15, 6, 10, 11, 7, 13, 0, 12, 1, 5, 8, 8, 12, 1, 11, 1, 3, 2, 4, 10, 7, 7, 7, 3, 10, 7, 2, 2, 3, 0, 1, 13, 5, 8, 2, 14, 0, 11, 13, 9, 3, 13, 2, 14, 2, 15, 4],
            [0, 0, 13, 6, 9, 12, 15, 7, 8, 0, 7, 4, 12, 15, 3, 2, 7, 1, 14, 4, 9, 3, 13, 12, 11, 12, 9, 9, 3, 7, 10, 9, 1, 9, 10, 2, 10, 14, 11, 0, 14, 4, 15, 12, 12, 9, 9, 8, 14, 1, 9, 14, 0, 6, 1, 0, 13, 9, 7, 6, 13, 2, 3, 9],
            [8, 0, 10, 13, 0, 7, 9, 7, 5, 1, 0, 3, 7, 10, 3, 15, 1, 15, 3, 11, 2, 6, 3, 10, 0, 10, 10, 3, 4, 15, 8, 6, 11, 11, 7, 5, 8, 5, 7, 15, 1, 11, 7, 13, 13, 6, 13, 13, 4, 2, 3, 15, 9, 5, 10, 6, 6, 6, 3, 11, 15, 13, 1, 15],
            [1, 1, 2, 10, 2, 2, 9, 5, 9, 2, 0, 1, 14, 2, 11, 6, 11, 6, 1, 0, 13, 7, 14, 1, 15, 14, 13, 7, 12, 11, 8, 11, 2, 11, 6, 10, 2, 3, 0, 0, 15, 0, 4, 6, 4, 12, 5, 5, 7, 14, 10, 6, 0, 3, 13, 0, 8, 1, 13, 10, 5, 1, 7, 5],
            [0, 5, 2, 12, 10, 2, 5, 1, 14, 0, 1, 4, 15, 11, 8, 7, 11, 14, 15, 6, 4, 1, 6, 6, 7, 13, 12, 5, 13, 2, 1, 6, 2, 13, 5, 15, 0, 8, 8, 6, 5, 5, 2, 0, 3, 13, 14, 2, 10, 5, 7, 6, 14, 5, 1, 4, 11, 2, 11, 1, 8, 15, 2, 4],
            [9, 9, 4, 5, 2, 5, 3, 12, 14, 5, 1, 3, 3, 0, 0, 6, 7, 14, 0, 15, 14, 11, 3, 10, 1, 9, 4, 14, 7, 14, 1, 0, 15, 11, 5, 9, 4, 0, 0, 10, 4, 4, 0, 7, 8, 15, 12, 8, 10, 8, 1, 2, 1, 11, 12, 14, 14, 14, 8, 10, 1, 5, 13, 10],
            [5, 10, 4, 4, 11, 10, 0, 6, 0, 12, 10, 5, 9, 11, 8, 10, 11, 3, 11, 14, 12, 9, 4, 6, 11, 12, 8, 7, 6, 14, 0, 6, 12, 4, 5, 3, 9, 0, 11, 6, 1, 3, 2, 12, 8, 9, 7, 12, 14, 7, 12, 6, 11, 13, 0, 2, 1, 3, 1, 8, 12, 2, 15, 15],
            [10, 11, 2, 3, 11, 10, 1, 7, 1, 10, 10, 14, 5, 13, 10, 3, 11, 15, 9, 14, 11, 11, 3, 15, 11, 6, 15, 13, 13, 1, 1, 10, 5, 1, 5, 11, 10, 3, 9, 12, 12, 1, 5, 6, 3, 3, 1, 1, 12, 8, 3, 15, 6, 2, 8, 14, 3, 4, 10, 9, 7, 13, 2, 6],
            [12, 0, 1, 0, 4, 3, 3, 6, 8, 3, 1, 13, 6, 12, 1, 1, 1, 4, 12, 4, 4, 9, 9, 14, 15, 3, 6, 4, 11, 1, 12, 5, 6, 0, 10, 9, 1, 8, 14, 5, 2, 8, 4, 15, 12, 13, 7, 14, 12, 2, 6, 9, 4, 13, 0, 15, 10, 10, 6, 12, 7, 12, 9, 10],
            [0, 8, 5, 11, 12, 12, 11, 7, 2, 9, 2, 15, 1, 1, 0, 0, 6, 5, 10, 1, 11, 12, 8, 7, 1, 7, 10, 4, 2, 8, 2, 5, 1, 1, 2, 9, 2, 0, 3, 7, 5, 1, 5, 5, 3, 1, 4, 3, 14, 8, 11, 7, 8, 0, 2, 13, 3, 15, 1, 13, 14, 15, 11, 13],
            [8, 13, 5, 14, 2, 9, 9, 13, 15, 8, 2, 14, 4, 2, 6, 0, 1, 13, 10, 13, 6, 12, 15, 11, 6, 11, 9, 9, 2, 9, 6, 14, 2, 9, 12, 1, 13, 9, 5, 11, 10, 4, 4, 5, 8, 9, 13, 10, 9, 0, 5, 15, 4, 12, 7, 10, 6, 5, 5, 15, 8, 8, 11, 14],
            [6, 9, 6, 7, 1, 15, 0, 1, 4, 15, 5, 3, 10, 9, 15, 9, 14, 12, 7, 6, 3, 0, 12, 8, 12, 2, 11, 8, 11, 8, 1, 10, 10, 7, 7, 5, 3, 5, 1, 2, 13, 11, 2, 5, 2, 10, 10, 1, 14, 14, 8, 1, 11, 1, 2, 6, 15, 10, 8, 7, 10, 7, 0, 3],
            [12, 6, 11, 1, 1, 7, 8, 1, 5, 5, 8, 4, 6, 5, 6, 4, 2, 8, 4, 1, 0, 0, 14, 2, 10, 14, 14, 11, 2, 9, 14, 15, 12, 14, 9, 3, 7, 14, 4, 7, 12, 9, 3, 5, 1, 0, 12, 9, 10, 5, 11, 12, 10, 10, 6, 14, 6, 13, 13, 5, 5, 10, 13, 10],
            [12, 6, 13, 0, 8, 0, 10, 6, 15, 15, 7, 3, 0, 10, 13, 14, 10, 13, 5, 13, 15, 14, 3, 4, 10, 10, 9, 6, 6, 15, 2, 7, 0, 10, 6, 14, 2, 9, 11, 7, 5, 5, 13, 14, 11, 15, 9, 4, 2, 0, 15, 5, 4, 14, 14, 1, 3, 4, 5, 8, 1, 1, 10, 12],
            [2, 5, 0, 4, 11, 5, 5, 6, 10, 4, 6, 7, 10, 3, 0, 14, 14, 0, 12, 15, 11, 12, 13, 7, 6, 3, 9, 1, 9, 8, 8, 8, 4, 10, 3, 1, 7, 10, 3, 2, 12, 6, 15, 14, 0, 6, 8, 10, 1, 9, 12, 12, 15, 7, 1, 11, 15, 13, 0, 4, 10, 0, 12, 11],
            [8, 12, 14, 15, 14, 15, 10, 0, 2, 14, 3, 1, 2, 6, 0, 2, 1, 7, 9, 0, 15, 13, 5, 14, 6, 8, 15, 4, 15, 6, 10, 6, 15, 3, 12, 8, 5, 4, 10, 5, 3, 0, 4, 13, 10, 9, 8, 4, 6, 3, 9, 6, 12, 11, 9, 13, 8, 10, 9, 9, 8, 12, 1, 2],
            [11, 10, 15, 15, 5, 14, 15, 7, 5, 9, 14, 14, 7, 11, 6, 6, 3, 8, 2, 3, 4, 14, 11, 1, 12, 15, 11, 6, 0, 0, 13, 7, 14, 3, 12, 14, 0, 15, 6, 1, 11, 2, 11, 8, 3, 13, 4, 12, 10, 13, 7, 14, 9, 13, 3, 10, 2, 14, 13, 4, 12, 13, 14, 10],
            [1, 11, 2, 12, 1, 10, 7, 12, 3, 3, 14, 9, 1, 10, 0, 11, 8, 10, 12, 12, 4, 12, 2, 11, 5, 0, 3, 15, 8, 2, 14, 3, 10, 2, 1, 13, 6, 14, 0, 0, 8, 11, 6, 13, 15, 10, 12, 7, 7, 11, 14, 9, 2, 7, 6, 8, 14, 9, 14, 10, 11, 9, 9, 12],
            [5, 10, 14, 2, 1, 4, 11, 5, 10, 2, 13, 9, 6, 12, 11, 5, 13, 4, 5, 14, 8, 7, 15, 9, 8, 4, 5, 2, 9, 11, 5, 3, 12, 2, 6, 1, 7, 4, 11, 4, 15, 0, 5, 2, 13, 11, 11, 2, 15, 10, 0, 12, 5, 8, 10, 1, 4, 11, 3, 13, 11, 7, 9, 14],
            [9, 8, 10, 5, 0, 2, 5, 8, 7, 3, 3, 6, 11, 1, 13, 15, 4, 4, 11, 6, 2, 6, 13, 11, 2, 6, 9, 4, 5, 13, 12, 2, 8, 7, 7, 12, 14, 15, 5, 12, 7, 0, 15, 15, 0, 5, 15, 0, 3, 9, 10, 15, 9, 11, 10, 10, 5, 3, 9, 3, 12, 13, 0, 13],
            [1, 11, 15, 0, 10, 5, 3, 5, 6, 7, 1, 11, 4, 11, 4, 2, 5, 12, 2, 5, 5, 6, 1, 5, 14, 9, 1, 5, 14, 12, 6, 10, 0, 8, 5, 11, 11, 11, 12, 10, 8, 10, 10, 1, 14, 1, 0, 8, 4, 7, 0, 11, 3, 1, 11, 12, 11, 8, 14, 15, 9, 3, 1, 14],
            [14, 11, 12, 12, 4, 6, 8, 14, 15, 1, 11, 2, 13, 3, 6, 2, 7, 1, 8, 1, 4, 9, 11, 15, 8, 1, 10, 13, 4, 13, 2, 7, 7, 10, 5, 2, 12, 12, 12, 3, 10, 8, 2, 11, 0, 3, 8, 9, 4, 2, 15, 7, 15, 6, 4, 6, 12, 7, 14, 9, 9, 8, 14, 12],
            [15, 4, 8, 12, 11, 11, 9, 5, 0, 0, 7, 6, 10, 5, 8, 2, 5, 6, 14, 11, 13, 0, 13, 15, 5, 4, 9, 15, 13, 12, 14, 15, 10, 2, 3, 6, 10, 14, 1, 8, 6, 7, 10, 1, 14, 9, 12, 13, 7, 2, 12, 10, 6, 11, 15, 1, 15, 11, 13, 0, 6, 13, 7, 15],
            [3, 3, 12, 5, 14, 9, 14, 14, 8, 0, 9, 1, 2, 2, 14, 11, 7, 1, 3, 1, 14, 15, 12, 8, 14, 2, 4, 13, 10, 5, 10, 8, 1, 7, 6, 5, 4, 2, 11, 5, 4, 13, 14, 6, 13, 15, 6, 6, 7, 12, 11, 5, 13, 10, 9, 13, 9, 14, 5, 6, 7, 14, 11, 7],
            [14, 12, 11, 5, 0, 5, 10, 5, 7, 1, 7, 11, 1, 0, 13, 6, 5, 14, 3, 0, 5, 14, 6, 7, 8, 5, 8, 6, 6, 3, 6, 1, 8, 3, 10, 7, 15, 6, 11, 6, 6, 7, 13, 2, 2, 0, 0, 11, 1, 15, 2, 14, 5, 1, 4, 8, 0, 1, 8, 0, 1, 1, 2, 2],
            [10, 13, 13, 3, 15, 14, 9, 12, 15, 15, 8, 5, 8, 10, 5, 9, 6, 6, 7, 15, 1, 0, 14, 9, 1, 11, 6, 11, 13, 4, 6, 14, 9, 12, 13, 8, 14, 6, 14, 2, 3, 15, 4, 4, 14, 4, 9, 12, 8, 0, 9, 11, 13, 10, 8, 14, 3, 5, 7, 11, 6, 7, 15, 2],
            [9, 9, 11, 6, 11, 0, 5, 4, 8, 10, 8, 11, 2, 12, 8, 7, 11, 13, 6, 1, 13, 13, 11, 4, 5, 7, 7, 9, 6, 4, 12, 0, 11, 8, 6, 12, 11, 4, 15, 11, 12, 8, 11, 11, 1, 3, 6, 14, 9, 6, 7, 5, 0, 10, 3, 15, 13, 7, 0, 1, 13, 15, 1, 14],
            [10, 6, 8, 7, 3, 6, 9, 15, 1, 3, 10, 14, 9, 0, 0, 10, 0, 15, 2, 0, 0, 0, 6, 0, 13, 9, 9, 1, 8, 6, 13, 2, 1, 9, 14, 9, 1, 4, 8, 4, 2, 0, 8, 5, 0, 11, 12, 15, 13, 1, 14, 14, 15, 7, 8, 4, 4, 12, 1, 12, 8, 3, 9, 5],
            [12, 11, 1, 4, 10, 14, 8, 12, 2, 4, 15, 2, 9, 7, 7, 11, 15, 12, 10, 11, 7, 4, 13, 0, 8, 6, 8, 8, 10, 5, 5, 13, 3, 7, 9, 13, 13, 14, 6, 8, 1, 5, 7, 12, 4, 4, 6, 9, 13, 1, 6, 1, 6, 14, 5, 8, 2, 10, 4, 10, 1, 9, 6, 15],
            [4, 13, 4, 9, 6, 11, 1, 8, 7, 11, 11, 1, 3, 10, 12, 11, 1, 10, 6, 10, 0, 7, 3, 0, 0, 6, 3, 9, 2, 1, 4, 8, 2, 10, 2, 15, 9, 15, 14, 14, 15, 14, 3, 2, 7, 6, 6, 10, 8, 8, 4, 11, 1, 13, 6, 0, 2, 10, 0, 11, 15, 14, 6, 9],
            [15, 0, 12, 13, 0, 9, 10, 4, 11, 5, 10, 0, 8, 7, 3, 2, 12, 6, 3, 8, 5, 15, 14, 2, 13, 13, 6, 11, 5, 6, 9, 10, 14, 5, 14, 4, 9, 7, 5, 11, 13, 2, 7, 1, 14, 9, 0, 7, 8, 12, 11, 15, 2, 1, 5, 11, 3, 7, 5, 1, 6, 3, 8, 6],
            [0, 3, 8, 1, 4, 6, 3, 1, 3, 8, 2, 0, 15, 15, 14, 15, 13, 10, 11, 9, 2, 11, 5, 12, 3, 3, 0, 1, 5, 3, 11, 6, 10, 11, 8, 5, 7, 15, 4, 12, 8, 8, 12, 12, 12, 1, 9, 4, 11, 6, 10, 11, 1, 12, 8, 12, 5, 6, 1, 14, 2, 10, 3, 0],
            [10, 13, 6, 9, 11, 1, 4, 10, 0, 13, 8, 7, 4, 12, 15, 5, 14, 12, 6, 9, 0, 0, 10, 5, 13, 10, 15, 3, 0, 8, 7, 0, 9, 8, 10, 6, 11, 8, 10, 13, 11, 7, 5, 5, 9, 13, 1, 15, 0, 5, 15, 5, 4, 7, 9, 9, 15, 8, 2, 6, 3, 8, 5, 8],
            [14, 0, 6, 2, 4, 12, 2, 13, 6, 10, 5, 2, 2, 1, 6, 11, 1, 6, 9, 13, 0, 13, 9, 3, 12, 4, 3, 8, 7, 0, 9, 12, 0, 1, 7, 10, 10, 7, 3, 9, 13, 5, 15, 4, 13, 0, 8, 5, 4, 14, 11, 3, 3, 13, 15, 9, 9, 12, 9, 5, 2, 0, 1, 14],
            [4, 14, 13, 0, 14, 15, 11, 10, 11, 1, 3, 3, 9, 1, 12, 8, 6, 5, 15, 11, 1, 7, 5, 3, 8, 13, 0, 13, 11, 5, 8, 1, 8, 6, 13, 4, 13, 7, 12, 6, 5, 5, 7, 0, 12, 1, 1, 8, 1, 6, 4, 2, 8, 8, 15, 11, 11, 11, 4, 4, 4, 7, 13, 12],
            [14, 15, 10, 0, 4, 3, 1, 9, 13, 7, 9, 9, 15, 5, 0, 3, 9, 6, 4, 7, 13, 11, 3, 2, 7, 1, 6, 8, 13, 7, 10, 4, 3, 9, 5, 9, 2, 6, 10, 7, 9, 13, 2, 14, 2, 14, 7, 2, 14, 2, 8, 8, 0, 9, 0, 9, 12, 6, 7, 7, 6, 8, 12, 13],
            [5, 15, 8, 12, 11, 3, 13, 4, 5, 14, 10, 4, 15, 15, 1, 10, 9, 14, 6, 6, 4, 12, 4, 9, 12, 2, 15, 13, 2, 5, 12, 2, 3, 2, 15, 11, 12, 2, 6, 2, 11, 6, 7, 9, 12, 10, 5, 1, 1, 5, 9, 6, 14, 11, 3, 11, 6, 10, 11, 11, 0, 12, 15, 1],
            [12, 6, 8, 10, 2, 5, 7, 9, 8, 14, 15, 15, 13, 10, 15, 3, 10, 10, 6, 10, 14, 10, 7, 5, 3, 7, 6, 12, 11, 12, 8, 9, 12, 9, 15, 15, 15, 7, 8, 3, 15, 14, 1, 12, 0, 0, 4, 0, 9, 10, 8, 7, 14, 10, 8, 14, 6, 2, 8, 1, 11, 10, 0, 1],
            [12, 1, 2, 12, 7, 10, 4, 11, 5, 14, 10, 2, 2, 9, 4, 13, 3, 14, 3, 15, 5, 0, 14, 7, 7, 15, 6, 5, 2, 8, 15, 9, 6, 6, 13, 10, 9, 8, 6, 3, 14, 7, 12, 9, 7, 8, 13, 12, 14, 13, 6, 0, 5, 1, 9, 12, 14, 0, 11, 11, 6, 3, 11, 7],
            [15, 4, 8, 12, 8, 11, 4, 15, 1, 6, 2, 13, 1, 7, 7, 12, 0, 8, 14, 14, 10, 14, 0, 12, 0, 3, 3, 11, 7, 4, 2, 13, 0, 0, 11, 2, 5, 8, 12, 11, 6, 5, 6, 0, 0, 4, 0, 0, 1, 9, 9, 11, 3, 2, 13, 4, 13, 9, 15, 4, 7, 8, 3, 2],
            [3, 13, 8, 8, 12, 10, 5, 4, 7, 13, 10, 13, 14, 3, 2, 12, 11, 0, 9, 5, 6, 4, 14, 4, 6, 9, 2, 5, 10, 3, 9, 10, 5, 0, 12, 5, 15, 5, 15, 15, 2, 12, 3, 11, 0, 15, 9, 14, 1, 5, 6, 6, 14, 5, 8, 0, 5, 9, 3, 7, 7, 12, 15, 1],
            [1, 11, 7, 4, 13, 3, 0, 8, 11, 9, 15, 1, 4, 12, 2, 12, 10, 4, 14, 3, 9, 14, 14, 2, 3, 11, 12, 4, 5, 10, 6, 15, 2, 13, 13, 9, 9, 1, 11, 12, 12, 14, 1, 5, 15, 1, 7, 14, 12, 10, 11, 13, 13, 5, 2, 4, 7, 7, 9, 4, 14, 15, 13, 10],
            [14, 15, 9, 14, 9, 5, 13, 2, 0, 0, 14, 8, 6, 2, 0, 7, 11, 10, 2, 13, 2, 14, 9, 6, 4, 11, 5, 14, 6, 1, 6, 14, 6, 3, 9, 5, 2, 9, 3, 11, 1, 14, 5, 4, 12, 5, 3, 5, 11, 3, 11, 6, 13, 7, 13, 7, 4, 9, 4, 13, 8, 3, 5, 11],
            [13, 12, 12, 13, 8, 2, 4, 2, 10, 6, 3, 5, 7, 7, 6, 13, 8, 6, 15, 4, 12, 7, 15, 4, 3, 9, 8, 15, 0, 3, 12, 1, 9, 8, 13, 10, 15, 4, 14, 1, 6, 15, 0, 4, 8, 9, 3, 1, 3, 15, 5, 5, 1, 11, 11, 10, 11, 10, 8, 8, 5, 4, 13, 0],
            [8, 4, 15, 9, 14, 9, 5, 8, 8, 10, 5, 15, 9, 8, 12, 5, 11, 10, 2, 12, 13, 1, 0, 2, 6, 13, 11, 9, 12, 0, 5, 0, 11, 5, 14, 12, 3, 4, 2, 10, 3, 12, 5, 15, 4, 8, 14, 1, 0, 13, 9, 5, 2, 4, 13, 8, 2, 5, 8, 9, 15, 3, 5, 5],
            [0, 3, 3, 4, 6, 5, 5, 1, 3, 2, 14, 5, 10, 7, 15, 11, 7, 13, 15, 4, 0, 12, 9, 15, 12, 0, 3, 1, 14, 1, 12, 9, 13, 8, 9, 15, 12, 3, 5, 11, 3, 11, 4, 1, 9, 4, 13, 7, 4, 10, 6, 14, 13, 0, 9, 11, 15, 15, 3, 3, 13, 15, 10, 15],
        ]);
        let hash = Hash::from_le_bytes([
            82, 46, 212, 218, 28, 192, 143, 92, 213, 66, 86, 63, 245, 241, 155, 189, 73, 159, 229, 180, 202, 105, 159,
            166, 109, 172, 128, 136, 169, 195, 97, 41,
        ]);
        assert_eq!(test_matrix.heavy_hash(hash), expected_hash);
    }
    #[test]
    fn test_generate_matrix() {
        #[rustfmt::skip]
        let expected_matrix = Matrix([
            [4, 5, 4, 5, 4, 5, 4, 5, 4, 5, 4, 5, 4, 5, 4, 5, 15, 3, 15, 3, 15, 3, 15, 3, 15, 3, 15, 3, 15, 3, 15, 3, 2, 10, 2, 10, 2, 10, 2, 10, 2, 10, 2, 10, 2, 10, 2, 10, 14, 1, 2, 2, 14, 10, 4, 12, 4, 12, 10, 10, 10, 10, 10, 10],
            [9, 11, 1, 11, 1, 11, 9, 11, 9, 11, 9, 3, 12, 13, 11, 5, 15, 15, 5, 0, 6, 8, 1, 8, 6, 11, 15, 5, 3, 6, 7, 3, 2, 15, 14, 3, 7, 11, 14, 7, 3, 6, 14, 12, 3, 9, 5, 1, 1, 0, 8, 4, 10, 15, 9, 10, 6, 13, 1, 1, 7, 4, 4, 6],
            [2, 6, 0, 8, 11, 15, 4, 0, 5, 2, 7, 13, 15, 3, 11, 12, 6, 2, 1, 8, 13, 4, 11, 4, 10, 14, 13, 2, 6, 15, 10, 6, 6, 5, 6, 9, 3, 3, 3, 1, 9, 12, 12, 15, 6, 0, 1, 5, 7, 13, 14, 1, 10, 10, 5, 14, 4, 0, 12, 13, 2, 15, 8, 4],
            [8, 6, 5, 1, 0, 6, 4, 8, 13, 0, 8, 12, 7, 2, 4, 3, 10, 5, 9, 3, 12, 13, 2, 4, 13, 14, 7, 7, 9, 12, 10, 8, 11, 6, 14, 3, 12, 8, 8, 0, 2, 10, 0, 9, 1, 9, 7, 8, 5, 2, 9, 13, 15, 6, 13, 10, 1, 9, 1, 10, 6, 2, 10, 9],
            [4, 2, 6, 14, 4, 2, 5, 7, 15, 6, 0, 4, 11, 9, 12, 0, 3, 2, 0, 4, 10, 5, 12, 3, 3, 4, 10, 1, 0, 13, 3, 12, 15, 0, 7, 10, 2, 2, 15, 0, 2, 15, 8, 2, 15, 12, 10, 6, 6, 2, 13, 3, 8, 14, 3, 13, 10, 5, 4, 5, 1, 6, 5, 10],
            [0, 3, 13, 12, 11, 4, 11, 13, 1, 12, 4, 11, 15, 14, 13, 4, 7, 1, 3, 0, 10, 3, 8, 8, 1, 2, 5, 14, 4, 5, 14, 1, 1, 3, 3, 1, 5, 15, 7, 5, 11, 8, 8, 12, 10, 5, 7, 9, 2, 10, 13, 11, 4, 2, 12, 15, 10, 6, 6, 0, 6, 6, 3, 12],
            [9, 12, 3, 3, 5, 8, 12, 13, 7, 4, 5, 11, 4, 0, 7, 2, 2, 15, 12, 14, 12, 5, 4, 2, 8, 8, 8, 13, 6, 1, 1, 5, 0, 15, 12, 13, 8, 5, 0, 4, 13, 1, 6, 1, 12, 14, 1, 0, 13, 12, 10, 10, 1, 4, 13, 13, 8, 4, 15, 13, 6, 6, 14, 10],
            [14, 15, 8, 0, 7, 2, 5, 10, 5, 3, 12, 0, 11, 3, 4, 2, 8, 11, 6, 14, 14, 3, 3, 12, 3, 7, 6, 2, 6, 12, 15, 1, 1, 13, 0, 6, 9, 9, 7, 7, 13, 4, 4, 2, 15, 5, 2, 15, 13, 13, 10, 6, 9, 15, 2, 9, 6, 10, 6, 14, 14, 3, 5, 11],
            [6, 4, 7, 8, 11, 0, 13, 11, 0, 7, 0, 0, 13, 6, 3, 11, 15, 14, 10, 2, 7, 8, 13, 14, 8, 15, 10, 8, 14, 6, 10, 14, 3, 11, 5, 11, 13, 5, 3, 12, 3, 0, 2, 0, 6, 14, 4, 12, 4, 4, 8, 15, 7, 8, 12, 11, 3, 9, 5, 13, 10, 14, 13, 4],
            [10, 0, 0, 15, 1, 4, 13, 3, 15, 10, 2, 5, 11, 2, 9, 14, 7, 3, 2, 8, 6, 15, 0, 12, 1, 4, 1, 9, 3, 0, 15, 8, 9, 13, 0, 7, 9, 10, 6, 14, 3, 7, 9, 7, 4, 0, 11, 8, 4, 6, 5, 8, 8, 0, 5, 14, 7, 12, 12, 2, 5, 6, 5, 6],
            [12, 0, 0, 14, 8, 3, 0, 3, 13, 10, 5, 13, 5, 7, 2, 4, 13, 11, 3, 1, 11, 2, 14, 5, 10, 5, 5, 9, 12, 15, 12, 8, 1, 0, 11, 13, 8, 1, 1, 11, 10, 0, 11, 15, 13, 9, 12, 14, 5, 4, 5, 14, 2, 7, 2, 1, 4, 12, 11, 11, 9, 12, 11, 15],
            [3, 15, 9, 8, 13, 12, 15, 7, 8, 7, 14, 6, 10, 3, 0, 5, 2, 2, 6, 6, 3, 2, 5, 12, 11, 2, 10, 11, 13, 3, 9, 7, 7, 6, 8, 15, 14, 14, 11, 11, 9, 7, 1, 3, 8, 5, 11, 11, 1, 2, 15, 8, 13, 8, 11, 4, 1, 5, 3, 12, 5, 3, 7, 7],
            [13, 13, 2, 14, 4, 3, 15, 2, 0, 15, 1, 5, 4, 1, 5, 1, 4, 14, 5, 1, 11, 13, 15, 1, 3, 3, 5, 13, 14, 1, 0, 4, 6, 1, 15, 7, 7, 0, 15, 8, 15, 3, 14, 7, 7, 8, 12, 10, 2, 14, 9, 2, 11, 11, 7, 10, 4, 3, 12, 13, 4, 13, 0, 14],
            [12, 14, 15, 15, 2, 0, 0, 13, 4, 6, 4, 2, 14, 11, 5, 6, 14, 8, 14, 7, 13, 15, 6, 15, 7, 9, 1, 0, 11, 9, 9, 0, 2, 12, 8, 8, 14, 11, 7, 5, 3, 0, 11, 12, 9, 2, 8, 9, 0, 0, 9, 8, 9, 8, 2, 14, 12, 2, 0, 14, 13, 8, 4, 10],
            [7, 10, 1, 15, 12, 14, 7, 4, 7, 13, 4, 8, 13, 12, 1, 7, 10, 6, 5, 14, 14, 3, 14, 4, 11, 14, 6, 12, 15, 12, 15, 12, 4, 5, 9, 8, 7, 7, 3, 0, 5, 7, 3, 8, 4, 4, 7, 5, 6, 12, 13, 0, 12, 10, 2, 5, 14, 9, 6, 4, 13, 13, 14, 5],
            [14, 5, 8, 3, 4, 15, 13, 14, 14, 10, 7, 14, 15, 2, 11, 14, 13, 13, 12, 10, 6, 9, 5, 5, 6, 13, 15, 13, 7, 0, 15, 11, 4, 12, 15, 7, 7, 4, 3, 11, 8, 14, 5, 10, 2, 4, 4, 12, 3, 6, 1, 9, 15, 1, 1, 13, 7, 5, 0, 14, 15, 7, 8, 6],
            [1, 2, 10, 5, 2, 13, 1, 11, 15, 10, 4, 9, 9, 12, 14, 13, 3, 5, 0, 3, 7, 11, 10, 3, 12, 5, 10, 2, 13, 7, 1, 7, 13, 8, 2, 8, 3, 14, 10, 3, 5, 12, 0, 9, 3, 9, 11, 2, 10, 9, 0, 6, 4, 0, 1, 14, 11, 0, 8, 6, 1, 15, 3, 10],
            [13, 9, 0, 5, 8, 7, 12, 15, 10, 10, 5, 1, 1, 7, 6, 1, 14, 5, 15, 2, 3, 5, 3, 5, 7, 3, 7, 7, 1, 4, 3, 14, 5, 0, 12, 0, 12, 10, 10, 6, 12, 6, 3, 5, 5, 11, 10, 1, 11, 3, 13, 3, 9, 11, 1, 7, 14, 14, 0, 8, 15, 5, 2, 7],
            [8, 5, 11, 6, 15, 0, 1, 13, 1, 6, 7, 15, 4, 3, 14, 12, 9, 3, 11, 6, 4, 12, 1, 11, 6, 12, 5, 11, 1, 12, 2, 3, 1, 2, 11, 12, 0, 5, 11, 5, 3, 13, 11, 3, 11, 14, 10, 8, 3, 9, 4, 8, 13, 11, 9, 11, 2, 4, 12, 3, 0, 14, 7, 11],
            [10, 11, 4, 10, 7, 8, 3, 14, 15, 8, 15, 6, 9, 8, 5, 6, 12, 1, 15, 6, 5, 5, 14, 13, 2, 12, 14, 6, 5, 5, 14, 9, 1, 10, 11, 14, 8, 6, 14, 11, 1, 15, 6, 11, 11, 8, 1, 2, 8, 5, 4, 15, 6, 8, 0, 8, 0, 11, 0, 1, 0, 7, 8, 15],
            [0, 15, 5, 0, 11, 4, 4, 2, 0, 4, 8, 12, 2, 2, 0, 8, 1, 2, 6, 5, 6, 12, 3, 1, 12, 1, 6, 10, 2, 5, 0, 2, 0, 11, 8, 6, 13, 4, 14, 4, 15, 5, 8, 11, 9, 6, 2, 6, 9, 1, 4, 2, 14, 10, 4, 4, 1, 1, 11, 8, 6, 11, 11, 9],
            [7, 3, 6, 5, 9, 1, 11, 0, 15, 13, 13, 13, 4, 14, 14, 12, 3, 7, 9, 3, 1, 6, 5, 9, 7, 6, 2, 11, 10, 4, 11, 14, 10, 13, 11, 8, 11, 8, 1, 15, 5, 0, 10, 5, 6, 0, 5, 15, 11, 6, 6, 4, 10, 11, 8, 12, 0, 10, 11, 11, 11, 1, 13, 6],
            [7, 15, 0, 0, 11, 5, 7, 13, 3, 7, 3, 2, 5, 12, 6, 11, 14, 4, 9, 8, 9, 9, 13, 0, 15, 2, 13, 2, 15, 6, 15, 1, 1, 7, 4, 0, 10, 1, 8, 14, 0, 10, 12, 4, 5, 13, 9, 0, 7, 12, 13, 11, 11, 8, 8, 15, 2, 15, 4, 4, 9, 3, 10, 7],
            [0, 9, 3, 5, 14, 6, 7, 14, 7, 2, 13, 7, 3, 15, 9, 15, 2, 8, 0, 4, 6, 0, 15, 6, 2, 1, 14, 8, 5, 8, 2, 4, 2, 11, 9, 2, 15, 13, 11, 12, 8, 15, 3, 13, 2, 2, 10, 13, 1, 8, 7, 15, 13, 6, 7, 7, 4, 3, 14, 7, 0, 9, 15, 11],
            [8, 13, 7, 7, 8, 8, 7, 8, 1, 4, 10, 1, 12, 4, 14, 11, 7, 12, 15, 0, 10, 15, 9, 2, 14, 2, 14, 2, 4, 5, 13, 3, 2, 10, 0, 15, 7, 6, 8, 11, 7, 6, 10, 10, 4, 7, 10, 6, 6, 14, 10, 4, 14, 6, 12, 2, 8, 1, 9, 13, 3, 4, 3, 14],
            [10, 10, 6, 3, 8, 5, 10, 7, 11, 10, 9, 4, 8, 14, 9, 10, 0, 9, 8, 14, 11, 15, 8, 13, 13, 7, 13, 13, 13, 9, 12, 11, 6, 3, 9, 6, 0, 0, 6, 6, 11, 6, 4, 8, 1, 5, 1, 7, 9, 6, 13, 4, 3, 8, 8, 11, 9, 10, 6, 11, 12, 13, 14, 14],
            [14, 10, 0, 15, 14, 4, 3, 0, 12, 4, 0, 14, 11, 9, 0, 6, 4, 6, 0, 9, 8, 14, 4, 4, 6, 8, 2, 8, 10, 3, 8, 0, 1, 1, 15, 4, 2, 4, 13, 9, 9, 4, 0, 5, 5, 1, 2, 5, 11, 6, 2, 1, 7, 8, 10, 10, 1, 5, 8, 6, 7, 0, 4, 14],
            [0, 15, 10, 11, 13, 12, 7, 7, 4, 0, 9, 5, 2, 8, 0, 10, 6, 6, 7, 5, 6, 7, 9, 0, 1, 4, 8, 14, 10, 3, 5, 5, 11, 5, 1, 10, 6, 10, 0, 14, 1, 15, 11, 12, 8, 2, 7, 8, 4, 0, 3, 11, 9, 15, 3, 5, 15, 15, 14, 15, 3, 4, 5, 14],
            [5, 12, 12, 8, 0, 0, 14, 1, 4, 15, 3, 2, 2, 6, 1, 10, 7, 10, 14, 5, 14, 0, 8, 5, 9, 0, 12, 8, 9, 10, 3, 12, 3, 2, 0, 0, 12, 12, 7, 13, 2, 6, 4, 7, 10, 10, 14, 1, 11, 6, 10, 3, 12, 2, 1, 10, 7, 13, 10, 12, 14, 11, 14, 8],
            [9, 5, 3, 12, 4, 3, 10, 14, 7, 5, 11, 12, 2, 13, 9, 8, 5, 2, 6, 2, 4, 9, 10, 10, 4, 3, 4, 0, 11, 1, 10, 9, 4, 10, 4, 5, 8, 11, 1, 7, 13, 7, 6, 6, 3, 12, 0, 0, 15, 6, 12, 12, 13, 7, 14, 14, 11, 15, 7, 14, 12, 6, 15, 2],
            [15, 2, 0, 12, 15, 14, 8, 14, 7, 14, 0, 3, 3, 11, 12, 2, 3, 14, 13, 5, 12, 9, 6, 11, 7, 4, 5, 1, 7, 12, 0, 11, 1, 5, 6, 6, 8, 6, 12, 2, 12, 3, 10, 3, 4, 10, 3, 3, 3, 10, 10, 14, 3, 13, 15, 0, 7, 6, 15, 6, 13, 7, 4, 11],
            [11, 15, 5, 14, 0, 1, 1, 14, 2, 3, 15, 14, 4, 3, 11, 1, 6, 6, 0, 12, 3, 5, 15, 6, 3, 11, 13, 11, 7, 7, 8, 11, 5, 9, 10, 10, 9, 14, 7, 1, 7, 2, 8, 6, 6, 5, 1, 9, 6, 5, 8, 14, 2, 14, 2, 9, 3, 3, 4, 15, 13, 5, 2, 7],
            [7, 8, 13, 9, 15, 8, 11, 7, 1, 9, 15, 12, 6, 9, 3, 1, 10, 10, 11, 0, 0, 8, 14, 5, 11, 12, 14, 4, 3, 9, 12, 9, 14, 0, 0, 9, 12, 4, 1, 13, 3, 6, 3, 4, 13, 10, 2, 9, 3, 7, 7, 10, 7, 10, 10, 3, 5, 15, 8, 9, 11, 7, 1, 14],
            [5, 5, 9, 1, 15, 3, 3, 11, 6, 11, 13, 13, 4, 12, 7, 12, 4, 8, 14, 13, 7, 12, 13, 8, 10, 2, 1, 12, 11, 7, 0, 8, 10, 9, 15, 1, 3, 9, 10, 0, 9, 1, 14, 1, 1, 9, 2, 2, 8, 9, 5, 6, 3, 2, 15, 9, 15, 6, 3, 11, 14, 4, 0, 4],
            [9, 2, 10, 2, 0, 9, 6, 13, 13, 0, 13, 14, 3, 12, 1, 15, 9, 3, 12, 2, 5, 15, 6, 6, 15, 11, 7, 11, 0, 4, 0, 11, 10, 12, 7, 9, 3, 0, 2, 2, 13, 13, 9, 6, 9, 2, 6, 4, 3, 6, 5, 10, 10, 9, 7, 2, 4, 9, 13, 11, 2, 13, 6, 8],
            [13, 15, 9, 8, 6, 2, 3, 2, 2, 12, 5, 3, 8, 6, 11, 6, 15, 7, 10, 3, 15, 8, 7, 5, 3, 8, 4, 2, 11, 1, 0, 4, 1, 1, 6, 1, 13, 6, 5, 1, 2, 6, 7, 10, 4, 3, 10, 6, 2, 0, 7, 13, 15, 1, 13, 0, 12, 10, 15, 6, 2, 4, 14, 3],
            [5, 11, 14, 4, 0, 7, 12, 4, 4, 14, 12, 3, 4, 10, 7, 14, 6, 4, 14, 7, 0, 12, 5, 9, 15, 6, 15, 6, 3, 12, 0, 10, 11, 7, 1, 14, 13, 5, 1, 14, 5, 15, 12, 1, 9, 13, 9, 13, 14, 5, 10, 11, 12, 10, 15, 11, 9, 13, 2, 14, 9, 12, 2, 11],
            [2, 12, 5, 7, 1, 5, 2, 11, 8, 4, 15, 6, 9, 14, 5, 1, 15, 4, 3, 1, 11, 4, 2, 1, 4, 5, 4, 4, 7, 3, 3, 12, 4, 3, 2, 15, 13, 1, 14, 15, 1, 4, 6, 11, 13, 15, 6, 12, 12, 13, 6, 8, 10, 0, 10, 12, 1, 10, 3, 2, 9, 8, 2, 8],
            [10, 12, 12, 6, 8, 5, 4, 4, 5, 3, 6, 7, 15, 5, 10, 3, 8, 15, 14, 5, 6, 2, 14, 4, 1, 7, 1, 3, 12, 3, 12, 4, 10, 15, 6, 6, 0, 6, 6, 8, 6, 9, 5, 7, 5, 1, 9, 2, 4, 9, 0, 8, 1, 1, 14, 3, 7, 14, 8, 9, 0, 4, 11, 7],
            [13, 11, 14, 7, 0, 4, 0, 10, 12, 11, 10, 8, 6, 12, 13, 15, 9, 2, 14, 9, 3, 0, 12, 14, 11, 15, 4, 7, 15, 14, 4, 8, 15, 12, 9, 14, 7, 7, 9, 13, 14, 14, 4, 9, 13, 8, 1, 13, 6, 3, 12, 7, 0, 15, 6, 15, 7, 2, 3, 0, 9, 5, 13, 0],
            [3, 8, 12, 11, 5, 9, 9, 14, 8, 14, 14, 5, 9, 9, 12, 10, 3, 12, 13, 0, 0, 0, 6, 7, 12, 4, 2, 3, 8, 8, 9, 15, 11, 1, 12, 13, 10, 15, 11, 1, 2, 13, 10, 1, 7, 2, 7, 11, 8, 15, 7, 6, 4, 6, 5, 11, 11, 15, 2, 1, 11, 1, 1, 8],
            [10, 7, 7, 1, 4, 13, 9, 10, 2, 2, 3, 7, 12, 8, 5, 5, 5, 5, 3, 1, 5, 6, 8, 2, 8, 11, 5, 0, 4, 12, 12, 6, 7, 9, 14, 10, 11, 8, 0, 9, 11, 4, 14, 7, 7, 8, 2, 15, 12, 7, 4, 4, 13, 2, 0, 3, 14, 0, 1, 5, 2, 15, 7, 11],
            [3, 8, 10, 4, 1, 7, 3, 13, 5, 14, 0, 9, 3, 1, 0, 11, 2, 15, 4, 9, 6, 5, 14, 0, 2, 8, 1, 14, 7, 6, 1, 5, 5, 7, 2, 0, 5, 3, 4, 15, 13, 10, 9, 13, 13, 12, 5, 11, 11, 14, 13, 10, 8, 14, 0, 8, 1, 7, 2, 10, 12, 12, 1, 11],
            [11, 14, 4, 13, 3, 11, 10, 6, 15, 2, 5, 10, 14, 4, 13, 3, 12, 7, 12, 10, 4, 0, 0, 1, 14, 6, 1, 2, 2, 12, 9, 2, 3, 11, 1, 4, 10, 4, 4, 7, 7, 12, 4, 3, 12, 11, 9, 3, 15, 13, 6, 13, 7, 11, 5, 12, 5, 13, 15, 12, 0, 13, 12, 9],
            [8, 7, 2, 2, 5, 3, 10, 15, 10, 8, 1, 0, 4, 5, 7, 6, 15, 13, 2, 14, 6, 2, 9, 5, 9, 0, 5, 12, 8, 6, 4, 12, 6, 8, 14, 15, 7, 15, 11, 2, 2, 12, 7, 9, 7, 11, 15, 7, 0, 4, 5, 13, 7, 2, 5, 9, 0, 5, 7, 6, 7, 12, 4, 1],
            [11, 4, 2, 13, 6, 10, 9, 4, 12, 9, 9, 6, 4, 2, 14, 14, 9, 5, 5, 15, 15, 9, 8, 11, 4, 2, 8, 11, 14, 3, 8, 10, 14, 9, 6, 6, 4, 7, 11, 2, 3, 7, 5, 1, 14, 2, 9, 4, 0, 1, 10, 7, 6, 7, 1, 3, 13, 7, 3, 2, 12, 3, 6, 6],
            [11, 1, 14, 3, 14, 3, 9, 9, 0, 11, 14, 6, 14, 7, 14, 8, 4, 2, 5, 6, 13, 3, 4, 10, 8, 8, 10, 11, 5, 1, 15, 15, 7, 0, 4, 14, 15, 13, 14, 13, 3, 2, 1, 6, 0, 6, 6, 4, 15, 6, 0, 12, 5, 11, 1, 7, 3, 3, 13, 12, 12, 6, 3, 2],
            [7, 2, 10, 14, 14, 13, 4, 14, 10, 6, 0, 2, 7, 7, 2, 5, 14, 1, 5, 14, 15, 1, 2, 9, 2, 13, 1, 3, 6, 1, 3, 13, 10, 6, 11, 13, 1, 7, 13, 15, 2, 11, 9, 6, 13, 7, 9, 2, 3, 13, 10, 10, 6, 2, 5, 9, 1, 3, 0, 3, 1, 5, 3, 12],
            [11, 14, 4, 2, 10, 11, 15, 5, 9, 7, 8, 11, 10, 9, 5, 7, 14, 3, 12, 2, 7, 15, 12, 15, 4, 15, 12, 9, 2, 6, 6, 6, 8, 5, 0, 7, 14, 15, 14, 14, 3, 12, 7, 12, 2, 4, 1, 7, 1, 3, 4, 7, 1, 9, 11, 15, 15, 3, 7, 1, 10, 9, 14, 14],
            [4, 13, 11, 1, 9, 6, 5, 1, 11, 6, 6, 8, 3, 9, 8, 15, 13, 12, 3, 13, 5, 9, 10, 5, 12, 1, 15, 14, 12, 1, 10, 11, 5, 7, 3, 12, 9, 12, 0, 2, 2, 3, 14, 4, 2, 13, 1, 15, 11, 8, 3, 13, 0, 10, 5, 4, 6, 0, 14, 8, 1, 0, 6, 15],
            [15, 2, 0, 5, 2, 14, 9, 0, 10, 5, 12, 8, 5, 6, 0, 1, 9, 4, 4, 1, 4, 6, 14, 5, 3, 0, 2, 2, 14, 9, 7, 0, 2, 15, 12, 0, 10, 12, 9, 12, 15, 1, 9, 4, 15, 3, 0, 13, 0, 6, 5, 0, 2, 6, 11, 9, 13, 15, 6, 3, 5, 4, 0, 8],
            [4, 14, 8, 14, 13, 4, 4, 10, 6, 12, 15, 11, 7, 2, 15, 6, 9, 9, 1, 11, 13, 2, 7, 10, 4, 4, 5, 12, 14, 15, 8, 5, 6, 1, 11, 15, 4, 11, 5, 2, 5, 7, 3, 4, 5, 7, 3, 8, 10, 13, 7, 5, 6, 5, 10, 1, 12, 13, 3, 6, 2, 8, 7, 15],
            [3, 15, 4, 9, 14, 12, 6, 1, 7, 0, 7, 15, 10, 6, 5, 5, 15, 5, 9, 4, 7, 6, 14, 2, 1, 4, 10, 3, 12, 1, 7, 1, 0, 10, 2, 11, 14, 13, 7, 10, 5, 11, 5, 11, 15, 5, 0, 3, 15, 1, 2, 14, 13, 13, 10, 9, 15, 12, 10, 5, 2, 10, 0, 6],
            [4, 6, 5, 13, 11, 10, 15, 4, 2, 15, 13, 6, 7, 7, 4, 0, 4, 6, 7, 4, 9, 1, 6, 7, 6, 1, 4, 2, 0, 11, 6, 3, 14, 5, 9, 2, 2, 10, 1, 2, 13, 14, 4, 11, 4, 7, 12, 9, 8, 2, 2, 9, 5, 7, 9, 12, 8, 15, 0, 9, 12, 11, 1, 12],
            [11, 12, 11, 9, 8, 15, 4, 12, 13, 10, 6, 6, 6, 12, 3, 0, 6, 15, 15, 10, 6, 12, 5, 7, 10, 2, 7, 1, 6, 12, 9, 11, 11, 14, 1, 12, 15, 0, 6, 2, 12, 15, 4, 15, 14, 8, 3, 4, 15, 4, 13, 3, 14, 1, 3, 7, 6, 13, 9, 1, 0, 12, 4, 14],
            [12, 11, 13, 10, 10, 10, 3, 7, 12, 3, 13, 9, 6, 0, 12, 10, 4, 11, 5, 4, 11, 5, 7, 14, 6, 10, 12, 12, 13, 15, 12, 1, 13, 15, 15, 7, 1, 2, 8, 6, 1, 12, 12, 0, 4, 3, 3, 3, 7, 8, 9, 10, 7, 7, 0, 0, 11, 13, 15, 4, 9, 5, 10, 9],
            [6, 12, 3, 0, 9, 11, 6, 4, 9, 9, 1, 5, 9, 14, 3, 7, 15, 3, 5, 0, 5, 11, 7, 6, 13, 5, 10, 2, 12, 10, 2, 6, 0, 1, 1, 13, 9, 3, 11, 7, 8, 2, 10, 9, 13, 6, 6, 4, 12, 0, 3, 10, 9, 4, 15, 11, 14, 1, 9, 3, 0, 14, 6, 1],
            [11, 14, 10, 10, 11, 6, 4, 7, 10, 0, 7, 9, 3, 2, 13, 13, 9, 9, 2, 3, 3, 14, 10, 4, 14, 1, 10, 7, 14, 4, 9, 15, 3, 11, 5, 10, 7, 8, 3, 0, 1, 2, 2, 3, 12, 9, 6, 2, 11, 15, 3, 9, 3, 6, 8, 0, 4, 5, 7, 3, 0, 14, 7, 9],
            [4, 11, 13, 12, 6, 2, 3, 15, 15, 3, 5, 1, 0, 5, 10, 2, 5, 3, 7, 10, 15, 0, 5, 3, 2, 10, 12, 10, 8, 3, 9, 15, 5, 3, 7, 13, 5, 7, 13, 12, 5, 10, 2, 9, 10, 1, 9, 4, 14, 1, 10, 13, 1, 2, 2, 12, 5, 3, 14, 7, 7, 8, 13, 13],
            [10, 12, 11, 10, 0, 15, 4, 3, 0, 8, 3, 0, 15, 0, 3, 10, 10, 9, 15, 3, 13, 3, 8, 3, 8, 2, 14, 7, 1, 6, 13, 8, 2, 2, 12, 3, 3, 0, 10, 12, 0, 1, 1, 7, 5, 0, 13, 10, 7, 13, 9, 9, 13, 7, 0, 1, 0, 2, 14, 2, 13, 0, 8, 3],
            [11, 3, 11, 10, 12, 15, 11, 6, 14, 8, 8, 5, 7, 11, 3, 1, 13, 7, 13, 4, 15, 7, 2, 3, 8, 7, 3, 8, 9, 15, 10, 15, 9, 0, 5, 4, 1, 7, 13, 8, 2, 7, 1, 10, 1, 12, 12, 1, 7, 12, 13, 5, 14, 10, 9, 15, 12, 2, 10, 3, 10, 3, 9, 12],
            [9, 8, 11, 0, 5, 6, 1, 5, 9, 1, 0, 12, 12, 0, 12, 11, 2, 8, 4, 0, 1, 7, 7, 5, 1, 14, 1, 9, 13, 7, 2, 12, 8, 9, 12, 13, 1, 11, 5, 3, 12, 14, 15, 4, 9, 8, 12, 7, 11, 1, 3, 9, 11, 5, 7, 14, 4, 6, 12, 3, 4, 12, 7, 9],
            [10, 12, 2, 14, 14, 1, 11, 8, 3, 7, 13, 7, 2, 1, 14, 13, 7, 6, 15, 8, 15, 12, 13, 10, 11, 15, 4, 2, 6, 13, 12, 3, 2, 10, 15, 14, 10, 11, 8, 14, 9, 3, 12, 9, 15, 2, 14, 14, 5, 13, 7, 6, 2, 1, 1, 4, 1, 0, 13, 10, 1, 0, 2, 9],
            [10, 5, 11, 14, 12, 1, 12, 7, 12, 8, 10, 5, 6, 10, 0, 7, 5, 6, 11, 11, 13, 12, 0, 13, 0, 6, 11, 0, 14, 4, 2, 1, 12, 7, 1, 10, 7, 15, 5, 3, 14, 15, 1, 3, 1, 2, 10, 4, 11, 8, 2, 11, 2, 5, 5, 4, 15, 5, 10, 3, 1, 7, 2, 14],
        ]);
        let hash = Hash::from_le_bytes([42; 32]);
        let matrix = Matrix::generate(hash);
        assert_eq!(matrix, expected_matrix);
    }
}

#[cfg(all(test, feature = "bench"))]
mod benches {
    extern crate test;

    use self::test::{black_box, Bencher};
    use super::{Matrix, XoShiRo256PlusPlus};
    use crate::Hash;
    use rand::{thread_rng, Rng};

    #[bench]
    pub fn bench_compute_rank(bh: &mut Bencher) {
        let mut generator = XoShiRo256PlusPlus::new(Hash::from_le_bytes([42; 32]));
        let mut matrix = Matrix::rand_matrix_no_rank_check(&mut generator);
        bh.iter(|| {
            for _ in 0..10 {
                black_box(&mut matrix);
                black_box(matrix.compute_rank());
            }
        });
    }

    #[bench]
    pub fn bench_heavy_hash(bh: &mut Bencher) {
        let mut generator = XoShiRo256PlusPlus::new(Hash::from_le_bytes([42; 32]));
        let mut input = Hash::new(thread_rng().gen());
        let mut matrix = Matrix::rand_matrix_no_rank_check(&mut generator);
        bh.iter(|| {
            for _ in 0..10 {
                black_box(&mut matrix);
                black_box(&mut input);
                black_box(matrix.heavy_hash(input));
            }
        });
    }
}
