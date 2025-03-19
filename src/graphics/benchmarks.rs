#[cfg(test)]
mod benchmarks {
    extern crate test;
    use crate::graphics::galaxy_texture::get_texture;
    use crate::prelude::*;
    use test::Bencher;

    #[bench]
    fn bench_get_texture_parallel(b: &mut Bencher) {
        let config = GalaxyConfig::default();
        b.iter(|| {
            get_texture(&config);
        })
    }
}
