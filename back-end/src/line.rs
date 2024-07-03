use ::rand::distributions::uniform::{SampleRange, SampleUniform};
use mockall::automock;
use rand::rngs::ThreadRng;
use rand::Rng;

#[automock]
trait GetRandom {
    fn gen_range<T, R>(&mut self, range: R) -> T
    where
        T: SampleUniform + 'static,
        R: SampleRange<T> + 'static;
}

struct GenRange<R> {
    rng: R,
}

impl GetRandom for GenRange<ThreadRng> {
    fn gen_range<T, R>(&mut self, range: R) -> T
    where
        T: SampleUniform + 'static,
        R: SampleRange<T> + 'static,
    {
        self.rng.gen_range(range)
    }
}

pub fn randline(maxlen: usize) -> Vec<u8> {
    randline_from(
        GenRange {
            rng: ::rand::thread_rng(),
        },
        maxlen,
    )
}

fn randline_from(mut rng: impl GetRandom, maxlen: usize) -> Vec<u8> {
    // original did 3 + rand(s) % (maxlen - 2)
    // so if rand(2) was 47, maxlen 50, the outcome is 3 + (47 % 48)
    // we have a length of 50
    // with a range we don't need to do - 2
    let len = rng.gen_range(3..=maxlen);

    let mut buffer = vec![0u8; len];

    for l in buffer.iter_mut().take(len - 2) {
        // ASCII 32 .. (including) ASCII 126
        *l = rng.gen_range(32..=126);
    }

    buffer
        .get_mut(len - 2..len)
        .expect("minimum length is 3")
        .copy_from_slice(&[13u8, 10]);

    // ensure start doesn't begin with "SSH-"
    if buffer.starts_with(&[b'S', b'S', b'H', b'-']) {
        buffer[0] = b'X';
    }

    buffer
}

#[cfg(test)]
mod tests {
    use std::ops::RangeInclusive;

    use mockall_double::double;

    use crate::line::randline_from;
    #[double]
    use crate::line::GetRandom;

    #[test]
    fn test_randline() {
        // given
        // mock rng
        let mut mock_rng = GetRandom::new();

        // set random length to requested maximum length
        mock_rng
            .expect_gen_range::<usize, RangeInclusive<usize>>()
            .returning(|x| *x.end());

        // every byte will be 97, or 'a'
        mock_rng
            .expect_gen_range::<u8, RangeInclusive<u8>>()
            .return_const(b'a');

        let max_len = 50;
        let randline = randline_from(mock_rng, max_len);

        // then
        // did we get a line our length?
        assert_eq!(randline.len(), max_len);

        // since we mocked the Rng, we can make sure that the line is as we expect it is
        assert_eq!(randline[..(max_len - 2)], vec![b'a'; max_len - 2]);
    }

    #[test]
    fn test_randline_carriage_return_line_feed() {
        // given
        // mock rng
        let mut ctx = GetRandom::new();

        // set random length to requested maximum length
        ctx.expect_gen_range::<usize, RangeInclusive<usize>>()
            .returning(|x| *x.end());

        ctx.expect_gen_range::<u8, RangeInclusive<u8>>()
            .return_const(b'a');

        // when
        let max_len = 50;
        let randline = randline_from(ctx, max_len);

        // then
        // did we get a line our length?
        assert_eq!(randline.len(), max_len);

        // do we end in a CRLF linebreak?
        assert_eq!(randline[(max_len - 2)..], [b'\r', b'\n']);
    }

    #[test]
    fn test_randline_no_ssh_prefix() {
        // given
        // mock rng
        let mut ctx = GetRandom::new();

        // set random length to requested maximum length
        ctx.expect_gen_range::<usize, RangeInclusive<usize>>()
            .returning(|x| *x.end());

        let fake_randoms = [b'S', b'S', b'H', b'-'];

        ctx.expect_gen_range::<u8, RangeInclusive<u8>>()
            .times(1)
            .return_const(fake_randoms[0]);
        ctx.expect_gen_range::<u8, RangeInclusive<u8>>()
            .times(1)
            .return_const(fake_randoms[1]);
        ctx.expect_gen_range::<u8, RangeInclusive<u8>>()
            .times(1)
            .return_const(fake_randoms[2]);
        ctx.expect_gen_range::<u8, RangeInclusive<u8>>()
            .times(1)
            .return_const(fake_randoms[3]);

        let max_len = 6;
        let randline = randline_from(ctx, max_len);

        assert_eq!(randline.len(), max_len);

        let xsh = [b'X', b'S', b'H', b'-'];
        assert_eq!(randline[..xsh.len()], xsh);
    }
}
