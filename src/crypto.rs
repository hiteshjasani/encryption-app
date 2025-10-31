use std::iter::zip;

use aes_gcm::{
  aead::{Aead, KeyInit, OsRng},
  AeadCore, Aes256Gcm, Key,
};
use anyhow::anyhow;
use crypto_bigint::{NonZero, RandomMod, U128, U64};
// use thiserror::Error;
use tracing::error;


const KEY_WRAPPER_KEY: [u8; 32] = [44, 122, 25, 25, 157, 162, 122, 10, 189, 72, 169, 15, 91, 54, 194, 213, 145, 15, 10, 165, 181, 142, 49, 122, 201, 27, 157, 154, 45, 12, 75, 86];
/// aes 256 bit key length in bytes
#[allow(dead_code)]
const AES_256_LEN_BYTES: usize = 32;
/// aes_gcm generates 96bit (12 byte) nonce by default
const NONCE_LEN_BYTES: usize = 12;
/// aes_gcm uses a 128bit (16 byte) authentication tag (MAC)
#[allow(dead_code)]
const TAG_LEN_BYTES: usize = 16;
/// padding needed for encrypted aes data keys
#[allow(dead_code)]
const PADDING_FOR_SHAMIR_60: &'static str = "00000000";



// #[derive(Error, Debug)]
// pub enum SSSError {
//     #[error("Threshold ({k_thres:?}) must be less than number of shares ({n_shares:?})")]
//     ThresholdGreaterThanShares {
//       k_thres: u16,
//       n_shares: u16
//     },

//     #[error("{msg:?}")]
//     Unexpected {
//       msg: String
//     },

//     #[error("unknown SSS error")]
//     Unknown,
// }

// #[derive(Debug, Clone, Copy)]
// pub struct ShamirShare {
//   points: [Point; 4],
// }

/// Returns four arrays for the key (32 byte) and one for the encrypted data
/// Returns four arrays  - array 0 is the first 8 bytes of the key, array 1 is the second 8 bytes
/// Each array is the points (shares) for a portion of the AES-256 key
#[allow(dead_code)]
pub fn shamir_encrypt_embed_nonce(data: &[u8], n_shares: u16, k_thres: u16) -> anyhow::Result<(Vec<Point>, Vec<Point>, Vec<Point>, Vec<Point>, Vec<u8>)> {
  // Get 32 byte (256 bit) aes key
  let (aes_key, enc_data) = symmetric_encrypt_embed_nonce(data)?;
  // Turn 32 bytes into 64 character hex string. Each byte is represented by two hex characters
  let ks = hex::encode(&aes_key);

  println!("ks = {ks}");
  // println!("k0 = {}", &ks[0..16]);
  // println!("k1 = {}", &ks[16..32]);

  // Split up 64 hex characters (32 bytes) into 16 hex character (8 byte) blocks for shamir splitting
  let k0 = U64::from_be_hex(&ks[0..16]);
  let k1 = U64::from_be_hex(&ks[16..32]);
  let k2 = U64::from_be_hex(&ks[32..48]);
  let k3 = U64::from_be_hex(&ks[48..64]);

  let prime = non_zero_prime()
    .map_err(|e| anyhow!("Unable to generate non-zero prime: {e}"))?;

  let shares0 = make_shares(k0.into(), n_shares, k_thres, &prime)?;
  let shares1 = make_shares(k1.into(), n_shares, k_thres, &prime)?;
  let shares2 = make_shares(k2.into(), n_shares, k_thres, &prime)?;
  let shares3 = make_shares(k3.into(), n_shares, k_thres, &prime)?;

  Ok((shares0, shares1, shares2, shares3, enc_data))
}

#[allow(dead_code)]
pub fn shamir_decrypt_embed_nonce(data: &[u8], _n_shares: u16, _k_thres: u16,
  shares0: Vec<Point>, shares1: Vec<Point>, shares2: Vec<Point>, shares3: Vec<Point>)
   -> anyhow::Result<Vec<u8>> {
  let prime = non_zero_prime()
    .map_err(|e| anyhow!("Unable to generate non-zero prime: {e}"))?;
  
  let k0 = recover_secret(&shares0, &prime)?;
  let k1 = recover_secret(&shares1, &prime)?;
  let k2 = recover_secret(&shares2, &prime)?;
  let k3 = recover_secret(&shares3, &prime)?;

  let mut ks: String = String::with_capacity(512);
  ks.push_str(&k0.to_string());
  ks.push_str(&k1.to_string());
  ks.push_str(&k2.to_string());
  ks.push_str(&k3.to_string());

  // println!("ks = {ks}");

  let aes_key = hex::decode(&ks)
    .map_err(|e| anyhow!("Unable to decode aes_key: {e}"))?;

  let data = symmetric_decrypt_using_embedded_nonce(&aes_key, data)?;

  Ok(data)
}

/// Encrypt content with aes data key and then break into shamir shares (multi-part key)
/// Each array is the points (shares) for a portion of the AES-256 key
#[allow(dead_code)]
pub fn shamir_encrypt_embed_nonce_60_bytes(data: &[u8], n_shares: u16, k_thres: u16) -> anyhow::Result<(Vec<MultiPartyKey8Points>, Vec<u8>)> {
  // Get 32 byte (256 bit) aes key
  let (aes_key, enc_data) = symmetric_encrypt_embed_nonce(data)?;
  // 32 byte data key turns into 60 byte encrypted key
  let (aes_key, _, _, _) = wrap_data_key(aes_key.as_slice())?;
  // Turn 60 bytes into 120 character hex string. Each byte is represented by two hex characters
  let ks = hex::encode(&aes_key);

  // println!("ks ({}) = {ks}", ks.len());
  // println!("k0 = {}", &ks[0..16]);
  // println!("k1 = {}", &ks[16..32]);

  // Split up 120 hex characters (60 bytes) into 16 hex character (8 byte) blocks for shamir splitting
  // println!("k0");
  let k0 = U64::from_be_hex(&ks[0..16]);
  // println!("k1");
  let k1 = U64::from_be_hex(&ks[16..32]);
  // println!("k2");
  let k2 = U64::from_be_hex(&ks[32..48]);
  // println!("k3");
  let k3 = U64::from_be_hex(&ks[48..64]);
  // println!("k4");
  let k4 = U64::from_be_hex(&ks[64..80]);
  // println!("k5");
  let k5 = U64::from_be_hex(&ks[80..96]);
  // println!("k6");
  let k6 = U64::from_be_hex(&ks[96..112]);
  // println!("k7");
  // Since the last part is only 4 bytes, we have to pad it with 4 bytes of zeros;
  let k7 = U64::from_be_hex(&format!("{}{PADDING_FOR_SHAMIR_60}", &ks[112..120]));
  // println!("done with kx");

  let prime = non_zero_prime()
    .map_err(|e| anyhow!("Unable to generate non-zero prime: {e}"))?;

  // Each part (16 hex char) of the overall key (120 hex char) is broken into n_shares len of Points
  let shares0 = make_shares(k0.into(), n_shares, k_thres, &prime)
    .inspect_err(|e| error!("error gen shares0: {e}"))?;
  let shares1 = make_shares(k1.into(), n_shares, k_thres, &prime)
    .inspect_err(|e| error!("error gen shares1: {e}"))?;
  let shares2 = make_shares(k2.into(), n_shares, k_thres, &prime)
    .inspect_err(|e| error!("error gen shares2: {e}"))?;
  let shares3 = make_shares(k3.into(), n_shares, k_thres, &prime)
    .inspect_err(|e| error!("error gen shares3: {e}"))?;
  let shares4 = make_shares(k4.into(), n_shares, k_thres, &prime)
    .inspect_err(|e| error!("error gen shares4: {e}"))?;
  let shares5 = make_shares(k5.into(), n_shares, k_thres, &prime)
    .inspect_err(|e| error!("error gen shares5: {e}"))?;
  let shares6 = make_shares(k6.into(), n_shares, k_thres, &prime)
    .inspect_err(|e| error!("error gen shares6: {e}"))?;
  let shares7 = make_shares(k7.into(), n_shares, k_thres, &prime)
    .inspect_err(|e| error!("error gen shares7: {e}"))?;

  let n_shares = n_shares as usize;
  let mut keys: Vec<MultiPartyKey8Points> = Vec::with_capacity(n_shares);
  for (p0, (p1, (p2, (p3, (p4, (p5, (p6, p7))))))) in zip(shares0.into_iter(),
      zip(shares1.into_iter(), zip(shares2.into_iter(), zip(shares3.into_iter(),
      zip(shares4.into_iter(), zip(shares5.into_iter(), zip(shares6.into_iter(), shares7.into_iter()))))))) {
    keys.push(MultiPartyKey8Points { p0, p1, p2, p3, p4, p5, p6, p7 })
  }

  Ok((keys, enc_data))
}

/// Combine shamir shares (multi-part key), decrypt aes data key and then content
#[allow(dead_code)]
pub fn shamir_decrypt_embed_nonce_60_bytes(data: &[u8], _n_shares: u16, _k_thres: u16, keys: Vec<MultiPartyKey8Points>) -> anyhow::Result<Vec<u8>> {
  let mut shares0 = Vec::new();
  let mut shares1 = Vec::new();
  let mut shares2 = Vec::new();
  let mut shares3 = Vec::new();
  let mut shares4 = Vec::new();
  let mut shares5 = Vec::new();
  let mut shares6 = Vec::new();
  let mut shares7 = Vec::new();

  for mpkey in keys {
    shares0.push(mpkey.p0);
    shares1.push(mpkey.p1);
    shares2.push(mpkey.p2);
    shares3.push(mpkey.p3);
    shares4.push(mpkey.p4);
    shares5.push(mpkey.p5);
    shares6.push(mpkey.p6);
    shares7.push(mpkey.p7);
  }

  let prime = non_zero_prime()
    .map_err(|e| anyhow!("Unable to generate non-zero prime: {e}"))?;

  let k0 = recover_secret(&shares0, &prime)?;
  let k1 = recover_secret(&shares1, &prime)?;
  let k2 = recover_secret(&shares2, &prime)?;
  let k3 = recover_secret(&shares3, &prime)?;
  let k4 = recover_secret(&shares4, &prime)?;
  let k5 = recover_secret(&shares5, &prime)?;
  let k6 = recover_secret(&shares6, &prime)?;
  let k7 = recover_secret(&shares7, &prime)?;

  let mut ks: String = String::with_capacity(512);
  ks.push_str(&k0.to_string());
  ks.push_str(&k1.to_string());
  ks.push_str(&k2.to_string());
  ks.push_str(&k3.to_string());
  ks.push_str(&k4.to_string());
  ks.push_str(&k5.to_string());
  ks.push_str(&k6.to_string());
  let k7_str = k7.to_string();
  // we need to grab only the first 4 bytes of the last section as it has 4 bytes of padding
  ks.push_str(&k7_str[0..(k7_str.len() - PADDING_FOR_SHAMIR_60.len())]);

  println!("ks = {ks}");

  let aes_key = hex::decode(&ks)
    .map_err(|e| anyhow!("Unable to decode aes_key: {e}"))?;

  let aes_key = unwrap_data_key(aes_key.as_slice())?;

  let data = symmetric_decrypt_using_embedded_nonce(&aes_key, data)?;

  Ok(data)
}

// 12th mersenne prime - 2^127 - 1
// pub static PRIME: U128 = U128::from_be_hex("7FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
const PRIME: U128 = U128::from_u128(2u128.pow(127) - 1);

// Evaluates polynomial (coefficient tuple) at x, used to generate a shamir pool
// in make_random_shares below.
//
// poly is in the form a + bx + cx^2 ...
fn fun_of_x(poly: &Vec<U128>, x: &U128, prime: &NonZero<U128>) -> U128 {
  let mut accum = U128::ZERO;
  let mut x_power = U128::ONE;

  for coeff in poly {
    accum = accum.add_mod(&coeff.mul_mod(&x_power, prime), prime);
    x_power = x_power.mul_mod(&x, prime);
  }

  accum
}


// secret
// n_shares - break secret into number of shares
// k_thres - threshold number of shares to recombine secret
fn make_shares(secret: u64, n_shares: u16, k_thres: u16, prime: &NonZero<U128>) -> anyhow::Result<Vec<Point>> {
  if k_thres >= n_shares { return Err(anyhow!("threshold (k) greater than shares (n): {k_thres} > {n_shares}")); }

  let secret = U128::from_u64(secret);

  // poly is set up as a0 + a1*x + a2*x^2
  let mut poly = vec![secret];

  for _ in 1..k_thres {
    let mut t = U128::ZERO;

    while t == U128::ZERO {
      t = U128::random_mod(&mut OsRng, prime);
    }
    poly.push(t);
  }

  let mut points = Vec::new();
  for i in 1..=n_shares {
    let x = i;
    let y = fun_of_x(&poly, &U128::from_u16(i), prime);
    points.push(Point { x, y });
  }

  Ok(points)
}

fn non_zero_prime() -> anyhow::Result<NonZero<U128>> {
  NonZero::new(PRIME).into_option()
    .ok_or_else(|| anyhow!("Unable to create non-zero prime"))
}

fn recover_secret(shares: &Vec<Point>, prime: &NonZero<U128>) -> anyhow::Result<U64> {
  let mut ans = U128::ZERO;

  let mut xs: Vec<U128> = Vec::new();
  let mut ys: Vec<U128> = Vec::new();

  for point in shares.iter() {
    let x = point.x;
    let y = point.y;
    xs.push(U128::from_u16(x));
    ys.push(U128::from(y));
  }

  for i in 0..xs.len() {
    let l_i = lagrange_basis(&xs, i, prime)?;
    let y_i = ys.get(i)
      .ok_or_else(|| anyhow!("Could not get y_i[{}]", i))?;

    ans = ans.add_mod(&y_i.mul_mod(&l_i, prime), prime);
  }

  let ans: U64 = ans.resize();
  Ok(ans)
}

fn lagrange_basis(xs: &Vec<U128>, i: usize, prime: &NonZero<U128>) -> anyhow::Result<U128> {
  let mut numer = U128::ONE;
  let mut denom = U128::ONE;


  let x_i = xs.get(i)
    .ok_or_else(|| anyhow!("Unable to get x_i value"))?;

  for j in 0..xs.len() {
    if i == j { continue; }

    let x_j = xs.get(j)
      .ok_or_else(|| anyhow!("Unable to get x_j value"))?;

    numer = numer.mul_mod(&x_j.neg_mod(prime), prime);
    denom = denom.mul_mod(&x_i.sub_mod(&x_j, prime), prime);
  }

  let inv = Into::<Option<_>>::into(denom.inv_mod(prime))
    .ok_or_else(|| anyhow!("Unable to find modulo multiplicative inverse"))?;

  Ok(numer.mul_mod(&inv, prime))
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
  pub x: u16,
  pub y: U128
}

impl Point {
  pub const BIT_SIZE_IN_BYTES: usize = 18;

  pub fn encode(&self) -> Vec<u8> {
    let mut res: Vec<u8> = Vec::with_capacity(Self::BIT_SIZE_IN_BYTES);
    let mut sx = self.x.to_be_bytes().to_vec();
    let mut sy = self.y.to_be_bytes().to_vec();
    res.append(&mut sx);
    res.append(&mut sy);
    res
  }

  pub fn decode(bytes: &Vec<u8>) -> anyhow::Result<Self> {
    let mut sx = bytes.clone();
    let sy = sx.split_off(2);
    let ax: [u8; 2] = Self::vec_to_array(sx);
    let x = u16::from_be_bytes(ax);
    let y = U128::from_be_slice(sy.as_slice());

    Ok(Self { x, y })
  }

  #[allow(dead_code)]
  pub fn encode_to_string(&self) -> String {
    let mut res = String::with_capacity(260);
    let sx = hex::encode(self.x.to_be_bytes());
    let sy = hex::encode(self.y.to_be_bytes());
    res.push_str(&sx);
    res.push_str(&sy);
    res
  }

  #[allow(dead_code)]
  pub fn decode_from_string(s: String) -> anyhow::Result<Self> {
    let (sx, sy) = s.split_at(4);
    let sx = hex::decode(sx)
      .map_err(|e| anyhow!("Can't parse point.x: {e}"))?;
    let sy = hex::decode(sy)
      .map_err(|e| anyhow!("Can't parse point.y: {e}"))?;
    let ax: [u8; 2] = Self::vec_to_array(sx);
    let x = u16::from_be_bytes(ax);
    let y = U128::from_be_slice(sy.as_slice());

    Ok(Self { x, y })
  }

  fn vec_to_array<T, const N: usize>(v: Vec<T>) -> [T; N]
  where
      T: Copy, // Or Clone, depending on whether you need to copy or clone elements
  {
      v.try_into()
          .unwrap_or_else(|v: Vec<T>| {
              panic!("Expected a Vec of length {} but it was {}", N, v.len());
          })
  }
}

//
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MultiPartyKey8Points {
  pub p0: Point,
  pub p1: Point,
  pub p2: Point,
  pub p3: Point,
  pub p4: Point,
  pub p5: Point,
  pub p6: Point,
  pub p7: Point,
}

#[allow(dead_code)]
impl MultiPartyKey8Points {
  // chunk size is expect to be sizeof(Point)
  pub fn encode(&self, chunk_size: usize) -> Vec<u8> {
    let mut res: Vec<u8> = Vec::with_capacity(chunk_size * 8);
    let mut s0 = self.p0.encode();
    let mut s1 = self.p1.encode();
    let mut s2 = self.p2.encode();
    let mut s3 = self.p3.encode();
    let mut s4 = self.p4.encode();
    let mut s5 = self.p5.encode();
    let mut s6 = self.p6.encode();
    let mut s7 = self.p7.encode();
    res.append(&mut s0);
    res.append(&mut s1);
    res.append(&mut s2);
    res.append(&mut s3);
    res.append(&mut s4);
    res.append(&mut s5);
    res.append(&mut s6);
    res.append(&mut s7);
    res
  }

  // chunk size is expect to be sizeof(Point)
  pub fn decode(bytes: &Vec<u8>, chunk_size: usize) -> anyhow::Result<Self> {
    // fn get_chunk(parts: &mut std::slice::Chunks<'_, u8>, idx: usize) -> Result<Point> {
    //   Point::decode(&parts.nth(idx)
    //     .map(|x| x.to_vec())
    //     .ok_or_else(|| AppError::General(format!("Unable to get part[{idx}] of stream")))?)
    // }
    // let mut parts = bytes.chunks(chunk_size);
    // let p0 = get_chunk(&mut parts, 0)?;
    // let p1 = get_chunk(&mut parts, 1)?;
    // let p2 = get_chunk(&mut parts, 2)?;
    // let p3 = get_chunk(&mut parts, 3)?;
    // let p4 = get_chunk(&mut parts, 4)?;
    // let p5 = get_chunk(&mut parts, 5)?;
    // let p6 = get_chunk(&mut parts, 6)?;
    // let p7 = get_chunk(&mut parts, 7)?;

    fn parse_chunk(opt_x: Option<&[u8]>) -> anyhow::Result<Point> {
      if let Some(x) = opt_x {
        Point::decode(&x.to_vec())
      } else {
        Err(anyhow!("Unable to parse_chunk()"))
      }
    }

    let mut parts = bytes.chunks(chunk_size);
    let p0 = parse_chunk(parts.next())?;
    let p1 = parse_chunk(parts.next())?;
    let p2 = parse_chunk(parts.next())?;
    let p3 = parse_chunk(parts.next())?;
    let p4 = parse_chunk(parts.next())?;
    let p5 = parse_chunk(parts.next())?;
    let p6 = parse_chunk(parts.next())?;
    let p7 = parse_chunk(parts.next())?;

    Ok(Self { p0, p1, p2, p3, p4, p5, p6, p7 })
  }
}



// returns (aes key, encrypted data)
pub fn symmetric_encrypt_embed_nonce(data: &[u8]) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
  let key = Aes256Gcm::generate_key(OsRng);
  let cipher = Aes256Gcm::new(&key);
  let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
  let ciphertext = cipher.encrypt(&nonce, data)
    .map_err(|e| anyhow!("Unable to encrypt data: {e}"))?;
  let mut nonce_ciphertext = Vec::with_capacity(nonce.len() + ciphertext.len());
  nonce_ciphertext.extend(nonce.iter());
  nonce_ciphertext.extend(ciphertext);

  Ok((key.to_vec(), nonce_ciphertext))
}

#[allow(dead_code)]
pub fn symmetric_decrypt_using_embedded_nonce(key: &[u8], nonce_ciphertext: &[u8]) -> anyhow::Result<Vec<u8>> {
  let key: &Key<Aes256Gcm> = key.into();
  let cipher = Aes256Gcm::new(&key);
  let (nonce_bytes, ciphertext_bytes) = nonce_ciphertext.split_at(NONCE_LEN_BYTES);

  let data = cipher.decrypt(nonce_bytes.into(), ciphertext_bytes)
    .map_err(|e| anyhow!("Unable to base64 decode: {e}"))?;

  Ok(data)
}

/// symmetric encryption with data key encrypted as well
/// returns (aes key, encrypted data)
#[allow(dead_code)]
pub fn symmetric_encrypt_embed_nonce_enc_data_key(data: &[u8]) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
  let (data_key, enc_data) = symmetric_encrypt_embed_nonce(data)?;

  let (wrapped_key, _, _, _) = wrap_data_key(&data_key.as_slice())?;

  Ok((wrapped_key, enc_data))
}

/// symmetric decryption with data key encrypted as well
/// returns (aes key, encrypted data)
#[allow(dead_code)]
pub fn symmetric_decrypt_using_embedded_nonce_enc_data_key(wrapped_key: &[u8], nonce_ciphertext: &[u8]) -> anyhow::Result<Vec<u8>> {
  let data_key = unwrap_data_key(wrapped_key)?;

  let plaintext = symmetric_decrypt_using_embedded_nonce(data_key.as_slice(), nonce_ciphertext)?;

  Ok(plaintext)
}

/// Wrap a data key by encrypting it
///
/// Math:
/// Original key is 256 bits or 32 bytes, which is chunked into four 8 byte groups equivalent to four 64 bit groups
/// Nonce is 96 bits or 12 bytes
/// Tag (or MAC Message Authencation Code) is 128 bits or 16 bytes
/// Cipher text is original key length + tag length + nonce length
/// Cipher text len = 32 bytes + 16 + 12 = 60 bytes
///
/// If we want to chunk an encrypted data key we'd want to break it into multiple 8 byte sections
/// Originally we'd break the 32 bytes into four 8 byte sections
/// Now we'd break 60 bytes into seven 8 bytes sections and one 4 byte section
fn wrap_data_key(data_key: &[u8]) -> anyhow::Result<(Vec<u8>, usize, usize, usize)> {
  let key: &Key<Aes256Gcm> = &KEY_WRAPPER_KEY.into();
  let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

  let encrypter = Aes256Gcm::new(&key);
  let cipher = encrypter.encrypt(&nonce, data_key)
    .map_err(|e| anyhow!("Unable to wrap data key: {e}"))?;

  let nonce = nonce.to_vec();
  let mut cipher_text = Vec::with_capacity(nonce.len() + cipher.len());
  let nonce_len = nonce.len();
  let cipher_len = cipher.len();
  cipher_text.extend(nonce);
  cipher_text.extend(cipher);

  Ok((cipher_text, key.len(), nonce_len, cipher_len))
}

fn unwrap_data_key(data_key: &[u8]) -> anyhow::Result<Vec<u8>> {
  let key: &Key<Aes256Gcm> = &KEY_WRAPPER_KEY.into();
  let (nonce_bytes, ciphertext_bytes) = data_key.split_at(NONCE_LEN_BYTES);

  let decrypter = Aes256Gcm::new(&key);
  let data = decrypter.decrypt(nonce_bytes.into(), ciphertext_bytes)
    .map_err(|e| anyhow!("Unable to wrap data key: {e}"))?;

  Ok(data)
}


// #region --------  tests  --------
#[cfg(test)]
mod tests {
    use std::iter::repeat_n;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_roundtrip_symmetric() -> anyhow::Result<()> {
      let orig = b"hello world";

      let (aes_key, enc_bytes) = symmetric_encrypt_embed_nonce(orig)?;
    
      let act = symmetric_decrypt_using_embedded_nonce(&aes_key, &enc_bytes)?;

      assert_eq!(orig.to_vec(), act);
      Ok(())
    }

    #[test]
    fn test_roundtrip_symmetric_enc_data_key() -> anyhow::Result<()> {
      let orig = b"hello world";

      let (aes_key, enc_bytes) = symmetric_encrypt_embed_nonce_enc_data_key(orig)?;
    
      let act = symmetric_decrypt_using_embedded_nonce_enc_data_key(&aes_key, &enc_bytes)?;

      assert_eq!(orig.to_vec(), act);
      Ok(())
    }

    #[test]
    fn test_shamir_roundtrip() -> anyhow::Result<()> {
      let data0 = Vec::from(b"hello world");
      let n_shares = 4;
      let k_thres = 2;

      let (sh0, sh1, sh2, sh3, enc_data) = shamir_encrypt_embed_nonce(data0.as_slice(), n_shares, k_thres)?;

      let data1 = shamir_decrypt_embed_nonce(&enc_data, n_shares, k_thres, sh0, sh1, sh2, sh3)?;

      assert_eq!(data0, data1);

      Ok(())
    }

    #[test]
    fn test_point_str_roundtrip() -> anyhow::Result<()> {
      let pt = Point {
        x: 9,
        y: U128::from_u16(87)
      };

      let orig = pt.encode_to_string();
      assert_eq!("000900000000000000000000000000000057", orig.as_str());

      let act = Point::decode_from_string(orig)?;
      assert_eq!(pt, act);

      Ok(())
    }

    #[test]
    fn test_point_roundtrip() -> anyhow::Result<()> {
      let pt = Point {
        x: 9,
        y: U128::from_u16(87)
      };

      let orig = pt.encode();
      assert_eq!("000900000000000000000000000000000057", hex::encode(&orig));

      let act = Point::decode(&orig)?;
      assert_eq!(pt, act);

      Ok(())
    }

    #[test]
    fn test_wrap_key_roundtrip() -> anyhow::Result<()> {
      let key = Aes256Gcm::generate_key(OsRng).to_vec();
      let (wrapped_key, key_len, nonce_len, cipher_len) = wrap_data_key(key.as_slice())?;
      let unwrapped_key = unwrap_data_key(wrapped_key.as_slice())?;

      assert_eq!(AES_256_LEN_BYTES, key.len());
      assert_eq!(AES_256_LEN_BYTES, key_len);
      assert_eq!(NONCE_LEN_BYTES, nonce_len);
      assert_eq!(AES_256_LEN_BYTES + TAG_LEN_BYTES, cipher_len, "cipher not matching size");
      assert_eq!(key.len() + TAG_LEN_BYTES + NONCE_LEN_BYTES, wrapped_key.len(), "encrypting adds 28 bytes overhead");

      assert_ne!(key, wrapped_key);
      assert_eq!(key, unwrapped_key);

      Ok(())
    }

    #[test]
    fn test_data0_length() -> anyhow::Result<()> {
      let data = b"hello world";
      let (_key, enc_bytes) = symmetric_encrypt_embed_nonce(data)?;

      assert_eq!(data.len() + TAG_LEN_BYTES + NONCE_LEN_BYTES, enc_bytes.len(), "pre and post byte length mismatch");

      Ok(())
    }

    #[test]
    fn test_data1_length() -> anyhow::Result<()> {
      let data: Vec<u8> = repeat_n(0x15, 64).collect();
      let (_key, enc_bytes) = symmetric_encrypt_embed_nonce(data.as_slice())?;

      assert_eq!(data.len() + TAG_LEN_BYTES + NONCE_LEN_BYTES, enc_bytes.len(), "pre and post byte length mismatch");

      Ok(())
    }

    #[test]
    fn test_nonce_is_not_already_embedded() -> anyhow::Result<()> {
      let data: Vec<u8> = repeat_n(0x15, 64).collect();
      let (_key, enc_bytes) = symmetric_encrypt_embed_nonce(data.as_slice())?;

      assert_ne!(enc_bytes.as_slice()[0..NONCE_LEN_BYTES], enc_bytes.as_slice()[NONCE_LEN_BYTES..(2 * NONCE_LEN_BYTES)], "is the nonce already embedded?");

      Ok(())
    }

    #[test]
    fn test_shamir_60_roundtrip_test_0() -> anyhow::Result<()> {
      let secret = b"hello world how are you doing?";
      let n_shares = 4;
      let k_thres = 3;

      let (mp_keys, enc_data) = shamir_encrypt_embed_nonce_60_bytes(secret, n_shares, k_thres)?;
      assert_eq!(secret.len() + TAG_LEN_BYTES + NONCE_LEN_BYTES, enc_data.len());

      let mut decrypt_keys = mp_keys.clone();
      let _ = decrypt_keys.remove((n_shares - 1) as usize);

      let clear = shamir_decrypt_embed_nonce_60_bytes(enc_data.as_slice(), n_shares, k_thres, decrypt_keys)?;
      assert_eq!(secret.to_vec(), clear);

      Ok(())
    }

    #[test]
    fn test_shamir_60_roundtrip_test_1() -> anyhow::Result<()> {
      let secret = b"hello world how are you doing?";
      let n_shares = 4;
      let k_thres = 3;

      let (mp_keys, enc_data) = shamir_encrypt_embed_nonce_60_bytes(secret, n_shares, k_thres)?;
      assert_eq!(secret.len() + TAG_LEN_BYTES + NONCE_LEN_BYTES, enc_data.len());

      let mut decrypt_keys = mp_keys.clone();
      let one_key = decrypt_keys.remove((n_shares - 1) as usize);

      let one_key_enc = one_key.encode(Point::BIT_SIZE_IN_BYTES);
      assert_eq!(Point::BIT_SIZE_IN_BYTES * 8, one_key_enc.len());

      let clear = shamir_decrypt_embed_nonce_60_bytes(enc_data.as_slice(), n_shares, k_thres, decrypt_keys)?;
      assert_eq!(secret.to_vec(), clear);

      Ok(())
    }

    #[test]
    fn test_shamir_60_roundtrip_test_encode_decode() -> anyhow::Result<()> {
      let secret = b"hello world how are you doing?";
      let n_shares = 4;
      let k_thres = 3;

      let (mp_keys, enc_data) = shamir_encrypt_embed_nonce_60_bytes(secret, n_shares, k_thres)?;
      assert_eq!(secret.len() + TAG_LEN_BYTES + NONCE_LEN_BYTES, enc_data.len());

      let mut decrypt_keys = mp_keys.clone();
      let one_key = decrypt_keys.remove((n_shares - 1) as usize);

      let one_key_enc = one_key.encode(Point::BIT_SIZE_IN_BYTES);
      assert_eq!(Point::BIT_SIZE_IN_BYTES * 8, one_key_enc.len());
      let one_key_dec = MultiPartyKey8Points::decode(&one_key_enc, Point::BIT_SIZE_IN_BYTES)?;
      assert_eq!(one_key, one_key_dec);

      let clear = shamir_decrypt_embed_nonce_60_bytes(enc_data.as_slice(), n_shares, k_thres, decrypt_keys)?;
      assert_eq!(secret.to_vec(), clear);

      Ok(())
    }
  }

// #endregion ----------------
