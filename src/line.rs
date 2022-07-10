use rand::thread_rng;
use rand::Rng;
use std::mem::MaybeUninit;
use std::ptr::addr_of;
use std::slice;

pub(crate) fn randline(line: &mut [MaybeUninit<u8>], maxlen: usize) -> usize {
    let len = thread_rng().gen_range(3..=(maxlen - 2));

    for l in line.iter_mut().take(len - 2) {
        let v = thread_rng().gen_range(32..=(32 + 95));
        l.write(v);
    }

    line[len - 2].write(13);
    line[len - 1].write(10);

    let first_4 = unsafe {
        // MaybeUninit::slice_assume_init_ref(&line[..4])
        slice::from_raw_parts(addr_of!(line).cast::<u8>(), 4)
    };

    if *first_4 == [b'S', b'S', b'H', b'-'] {
        line[0].write(b'X');
    }

    len
}
