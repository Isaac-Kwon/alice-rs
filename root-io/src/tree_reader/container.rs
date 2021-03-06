use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom, Read};
use std::path::PathBuf;
use failure::Error;
use nom::*;

use core::*;

#[derive(Debug, Clone)]
pub(crate) enum Container {
    /// Decompressed content of a `TBasket`
    InMemory(Vec<u8>),
    /// Filename, Position, and len of a `TBasket` on disk
    OnDisk(PathBuf, SeekFrom, usize),
}

impl Container {
    /// Return the number of entries and the data; reading it from disk if necessary
    pub(crate) fn raw_data(self) -> Result<(u32, Vec<u8>), Error> {
        match self {
            Container::InMemory(buf) => {
                match tbasket2vec(buf.as_slice()) {
                    IResult::Done(_, v) => Ok(v),
                    _ => Err(format_err!("tbasket2vec parser failed"))
                }
            },
            Container::OnDisk(p, seek, len) => {
                let f = File::open(&p)?;
                let mut reader = BufReader::new(f);
                let mut buf = vec![0; len];
                reader.seek(seek)?;
                reader.read_exact(&mut buf)?;
                // println!("{:#?}", tbasket(buf.as_slice(), be_u32).unwrap().1);
                match tbasket2vec(buf.as_slice()) {
                    IResult::Done(_, v) => Ok(v),
                    _ => Err(format_err!("tbasket2vec parser failed"))
                }
            }
        }
    }
    // /// For debugging: Try to find the file of this container. Out of luck if the container was inlined
    // pub(crate) fn file(&self) -> Option<PathBuf> {
    //     match *self {
    //         // No file name available
    //         Container::InMemory(_) => None,
    //         Container::OnDisk(ref p, _, _) => Some(p.to_owned())
    //     }
    // }
}

/// Return a tuple indicating the number of elements in this basket
/// and the content as a Vec<u8>
fn tbasket2vec(input: &[u8]) -> IResult<&[u8], (u32, Vec<u8>)>
{
    do_parse!(input,
              hdr: tkey_header >>
              _ver: be_u16 >>
              _buf_size: be_u32 >>
              _entry_size: be_u32 >>
	      n_entry_buf: be_u32 >>
	      last: be_u32 >>
	      _flag: be_i8 >>
              buf: rest >>
              ({
                  let buf = if hdr.uncomp_len as usize > buf.len() {
                      (decompress(buf).unwrap().1)
                  } else {
                      buf.to_vec()
                  };
                  // Not the whole buffer is filled, no, no, no, that
                  // would be to easy! Its only filled up to `last`,
                  // whereby we have to take the key_len into account...
                  let useful_bytes = (last - hdr.key_len as u32) as usize;
                  (n_entry_buf, buf.as_slice()[..useful_bytes].to_vec())
              }))
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::{BufReader, SeekFrom, Read, Seek};
    use nom::*;
    use core::tkey_header;

    use super::tbasket2vec;

    #[test]
    fn basket_simple() {
        let path = "./src/test_data/simple.root";
        let f = File::open(&path).unwrap();
        let mut reader = BufReader::new(f);
        // Go to first basket
        reader.seek(SeekFrom::Start(218)).unwrap();
        // size from fbasketbytes
        let mut buf = vec![0; 86];
        // let mut buf = vec![0; 386];
        reader.read_exact(&mut buf).unwrap();

        println!("{}", buf.to_hex(16));
        println!("{:?}", tkey_header(&buf));
        // println!("{:#?}", tbasket(&buf, be_u32));
        println!("{:#?}", tbasket2vec(&buf));
    }

    /// Test the first basket of the "Tracks.fP[5]" branch
    #[test]
    #[ignore]
    fn basket_esd() {
        let path = "./src/test_data/AliESDs.root";
        let f = File::open(&path).unwrap();
        let mut reader = BufReader::new(f);
        // Go to first basket
        reader.seek(SeekFrom::Start(77881)).unwrap();
        // size from fbasketbytes
        let mut buf = vec![0; 87125];
        reader.read_exact(&mut buf).unwrap();

        println!("{:?}", tkey_header(&buf).unwrap().1);
        // println!("{:#?}", tbasket(&buf, |i| count!(i, be_f32, 15)).unwrap().1);
        println!("{:#?}", tbasket2vec(&buf));
    }
}
