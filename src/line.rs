use mockall::automock;
use mockall_double::double;

#[automock]
mod rand_wrap {
    use rand::distributions::uniform::{SampleRange, SampleUniform};
    use rand::Rng;

    #[cfg_attr(test, allow(dead_code))]
    pub(crate) fn rand_in_range<T, R>(range: R) -> T
    where
        T: SampleUniform + 'static,
        R: SampleRange<T> + 'static,
    {
        ::rand::thread_rng().gen_range(range)
    }
}

#[double]
use self::rand_wrap as rand;

pub(crate) fn randline(maxlen: usize) -> Vec<u8> {
    // original did 3 + rand(s) % (maxlen - 2)
    // so if rand(2) was 47, maxlen 50, the outcome is 3 + (47 % 48)
    // we have a length of 50
    // with a range we don't need to do - 2
    let len = rand::rand_in_range(3..=maxlen);

    let mut buffer = vec![0u8; len];

    for l in buffer.iter_mut().take(len - 2) {
        // ASCII 32 .. (including) ASCII 126
        *l = rand::rand_in_range(32..=126);
    }

    buffer[len - 2] = 13;
    buffer[len - 1] = 10;

    if len > 3 && buffer[0..4] == [b'S', b'S', b'H', b'-'] {
        buffer[0] = b'X';
    }

    buffer
}

#[cfg(test)]
mod tests {
    use std::{
        ops::RangeInclusive,
        sync::{Mutex, MutexGuard},
    };

    use mockall::lazy_static;

    use crate::line::mock_rand_wrap as rand;
    use crate::line::randline;

    lazy_static! {
        static ref MTX: Mutex<()> = Mutex::new(());
    }

    // When a test panics, it will poison the Mutex. Since we don't actually
    // care about the state of the data we ignore that it is poisoned and grab
    // the lock regardless.  If you just do `let _m = &MTX.lock().unwrap()`, one
    // test panicking will cause all other tests that try and acquire a lock on
    // that Mutex to also panic.
    fn get_lock(m: &'static Mutex<()>) -> MutexGuard<'static, ()> {
        match m.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    #[test]
    fn test_randline() {
        let _m = get_lock(&MTX);

        // given
        // mock rng
        let ctx = rand::rand_in_range_context();

        // set random length to requested maximum length
        ctx.expect::<usize, RangeInclusive<usize>>()
            .returning(|x| *x.end());

        // every byte will be 66, or 'b'
        ctx.expect::<u8, RangeInclusive<u8>>().return_const(65u8);

        let max_len = 50;
        let randline = randline(max_len);

        // then
        // did we get a line our length?
        assert_eq!(randline.len(), max_len);

        // since we mocked the Rng, we can make sure that the line is as we expect it is
        assert_eq!(randline[..(max_len - 2)], vec![b'a'; max_len - 2]);
    }

    #[test]
    fn test_randline_carriage_return_line_feed() {
        let _m = get_lock(&MTX);

        // given
        // mock rng
        let ctx = rand::rand_in_range_context();

        ctx.expect::<u8, RangeInclusive<u8>>().return_const(b'a');

        // when
        let max_len = 50;
        let randline = randline(max_len);

        // then
        // did we get a line our length?
        assert_eq!(randline.len(), max_len);

        // do we end in a CRLF linebreak?
        assert_eq!(randline[(max_len - 2)..], [b'\r', b'\n']);
    }

    #[test]
    fn test_randline_no_ssh_prefix() {
        let _m = get_lock(&MTX);

        // given
        // mock rng
        let ctx = rand::rand_in_range_context();

        // set random length to requested maximum length
        ctx.expect::<usize, RangeInclusive<usize>>()
            .returning(|x| *x.end());

        let fake_randoms = [b'S', b'S', b'H', b'-'];

        ctx.expect::<u8, RangeInclusive<u8>>()
            .times(1)
            .return_const(fake_randoms[0]);
        ctx.expect::<u8, RangeInclusive<u8>>()
            .times(1)
            .return_const(fake_randoms[1]);
        ctx.expect::<u8, RangeInclusive<u8>>()
            .times(1)
            .return_const(fake_randoms[2]);
        ctx.expect::<u8, RangeInclusive<u8>>()
            .times(1)
            .return_const(fake_randoms[3]);

        let max_len = 6;
        let randline = randline(max_len);

        assert_eq!(randline.len(), max_len);

        let xsh = [b'X', b'S', b'H', b'-'];
        assert_eq!(randline[..xsh.len()], xsh);
    }
}
