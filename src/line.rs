use mockall::automock;
use mockall_double::double;

#[automock]
mod rand {
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
use self::rand as rng;

pub(crate) fn randline(maxlen: usize) -> Vec<u8> {
    let len = rng::rand_in_range(3..=maxlen);
    println!("Len = {}", len);

    let mut buffer = vec![0u8; len];

    for l in buffer.iter_mut().take(len - 2) {
        *l = rng::rand_in_range(32..=(32 + 95));
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
    use std::ops::RangeInclusive;

    use crate::line::{mock_rand, randline};

    #[test]
    fn test_randline() {
        // mock rng
        let ctx = mock_rand::rand_in_range_context();

        // set random length to requested maximum length
        ctx.expect::<usize, RangeInclusive<usize>>()
            .returning(|x| *x.end());

        // every byte will be 66, or 'b'
        ctx.expect::<u8, RangeInclusive<u8>>().return_const(65u8);

        let max_len = 50;
        let randline = randline(max_len);
        assert_eq!(randline.len(), max_len);

        // since we mocked the Rng, we can make sure that the line is as we expect it is
        assert_eq!(randline[..(max_len - 2)], vec![65u8; max_len - 2]);

        // do we end in a Windows linebreak?
        assert_eq!(randline[(max_len - 2)..], [13u8, 10u8]);
    }

    #[test]
    fn test_randline_no_ssh_prefix() {
        // mock rng
        let ctx = mock_rand::rand_in_range_context();

        // set random length to requested maximum length
        ctx.expect::<usize, RangeInclusive<usize>>()
            .returning(|x| *x.end());

        let fake_randoms = [b'S', b'S', b'H', b'-'];

        let mut i = 0;

        ctx.expect::<u8, RangeInclusive<u8>>().returning(move |_| {
            let l = fake_randoms[i];
            i += 1;
            l
        });

        let max_len = 6;

        let randline = randline(max_len);
        assert_eq!(randline.len(), max_len);

        let xsh = [b'X', b'S', b'H', b'-'];
        assert_eq!(randline[..xsh.len()], xsh);

        // do we end in a Windows linebreak?
        assert_eq!(randline[(max_len - 2)..], [13u8, 10u8]);
    }
}
