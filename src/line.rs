use rand::thread_rng;
use rand::Rng;

pub(crate) fn randline(maxlen: usize) -> Vec<u8> {
    let len = thread_rng().gen_range(3..=(maxlen - 2));

    let mut buffer = vec![0u8; len];

    for l in buffer.iter_mut().take(len - 2) {
        *l = thread_rng().gen_range(32..=(32 + 95));
    }

    buffer[len - 2] = 13;
    buffer[len - 1] = 10;

    if len > 3 && buffer[0..4] == [b'S', b'S', b'H', b'-'] {
        buffer[0] = b'X';
    }

    buffer
}
