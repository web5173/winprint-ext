use windows::Win32::System::Com::{IStream, STREAM_SEEK_END, STREAM_SEEK_SET};

pub fn read_com_stream(stream: &IStream) -> Result<Vec<u8>, windows::core::Error> {
    let mut size = 0;
    unsafe {
        stream.Seek(0, STREAM_SEEK_END, Some(&mut size))?;
        stream.Seek(0, STREAM_SEEK_SET, None)?;
        let mut data = vec![0u8; size as usize];
        let mut total = 0u64;
        while total < size {
            let mut n_read = 0u32;
            stream
                .Read(
                    data.as_mut_ptr().add(total as usize) as *mut _,
                    (size - total) as u32,
                    Some(&mut n_read),
                )
                .ok()?;
            if n_read == 0 {
                break;
            }
            total += n_read as u64;
        }
        data.truncate(total as usize);
        Ok(data)
    }
}

/// Copies the contents of a COM stream to a vector.
pub fn copy_com_stream_to_vec(
    dest: &mut Vec<u8>,
    stream: &IStream,
) -> Result<(), windows::core::Error> {
    let mut size = 0;
    unsafe {
        stream.Seek(0, STREAM_SEEK_END, Some(&mut size))?;
        stream.Seek(0, STREAM_SEEK_SET, None)?;
        dest.clear();
        dest.resize(size as usize, 0);
        let mut total = 0u64;
        while total < size {
            let mut n_read = 0u32;
            stream
                .Read(
                    dest.as_mut_ptr().add(total as usize) as *mut _,
                    (size - total) as u32,
                    Some(&mut n_read),
                )
                .ok()?;
            if n_read == 0 {
                break;
            }
            total += n_read as u64;
        }
        dest.truncate(total as usize);
        Ok(())
    }
}
