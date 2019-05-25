use core::arch::x86_64::*;

macro_rules! unroll4 {
    ($e:expr) => {{
        $e;
        $e;
        $e;
        $e;
    }};
}

#[inline(always)]
unsafe fn hadd_ps(mut v: __m256) -> f32 {
    v = _mm256_hadd_ps(v, v);
    v = _mm256_hadd_ps(v, v);
    let v = std::mem::transmute::<__m256, [f32; 8]>(v);
    v[0] + v[4]
}

static SABS_MASK: u32 = 0x7FFF_FFFF;

const STEP: usize = 8 * 4;

pub fn srotg(a: f32, b: f32) -> (f32, f32, f32, f32) {
    if a == 0.0 && b == 0.0 {
        return (0.0, 0.0, 1.0, 0.0);
    }
    let h = a.hypot(b);
    let r = if a.abs() > b.abs() {
        h * a.signum()
    } else {
        h * b.signum()
    };
    let c = a / r;
    let s = b / r;
    let z = if a.abs() > b.abs() {
        s
    } else if c != 0.0 {
        1.0 / c
    } else {
        1.0
    };
    (r, z, c, s)
}

pub unsafe fn srot(
    n: usize,
    mut x: *mut f32,
    incx: isize,
    mut y: *mut f32,
    incy: isize,
    c: f32,
    s: f32,
) {
    if incx == 1 && incy == 1 {
        let cv = _mm256_broadcast_ss(&c);
        let sv = _mm256_broadcast_ss(&s);

        for _ in 0..n / STEP {
            unroll4!({
                let xv = _mm256_loadu_ps(x);
                let yv = _mm256_loadu_ps(y);

                _mm256_storeu_ps(x, _mm256_fmadd_ps(cv, xv, _mm256_mul_ps(sv, yv)));
                _mm256_storeu_ps(y, _mm256_fmsub_ps(cv, yv, _mm256_mul_ps(sv, xv)));

                x = x.offset(8);
                y = y.offset(8);
            });
        }
        for _ in 0..n % STEP {
            let xi = *x;
            let yi = *y;

            *x = c * xi + s * yi;
            *y = c * yi - s * xi;

            x = x.offset(1);
            y = y.offset(1);
        }
    } else {
        for _ in 0..n {
            let xi = *x;
            let yi = *y;

            *x = c * xi + s * yi;
            *y = c * yi - s * xi;

            x = x.offset(incx);
            y = y.offset(incy);
        }
    }
}

pub unsafe fn sswap(n: usize, mut x: *mut f32, incx: isize, mut y: *mut f32, incy: isize) {
    if incx == 1 && incy == 1 {
        for _ in 0..n / STEP {
            unroll4!({
                let xv = _mm256_loadu_ps(x);
                let yv = _mm256_loadu_ps(y);
                _mm256_storeu_ps(x, yv);
                _mm256_storeu_ps(y, xv);
                x = x.offset(8);
                y = y.offset(8);
            });
        }
        for _ in 0..n % STEP {
            let xi = *x;
            let yi = *y;

            *x = yi;
            *y = xi;

            x = x.offset(1);
            y = y.offset(1);
        }
    } else {
        for _ in 0..n {
            let xi = *x;
            let yi = *y;

            *x = yi;
            *y = xi;

            x = x.offset(incx);
            y = y.offset(incy);
        }
    }
}

pub unsafe fn sscal(n: usize, a: f32, mut x: *mut f32, incx: isize) {
    if incx == 1 {
        let av = _mm256_broadcast_ss(&a);
        for _ in 0..n / STEP {
            unroll4!({
                _mm256_storeu_ps(x, _mm256_mul_ps(av, _mm256_loadu_ps(x)));
                x = x.offset(8);
            });
        }
        for _ in 0..n % STEP {
            *x *= a;
            x = x.offset(1);
        }
    } else {
        for _ in 0..n {
            *x *= a;
            x = x.offset(incx);
        }
    }
}

pub unsafe fn scopy(n: usize, mut x: *const f32, incx: isize, mut y: *mut f32, incy: isize) {
    if incx == 1 && incy == 1 {
        for _ in 0..n / STEP {
            unroll4!({
                _mm256_storeu_ps(y, _mm256_loadu_ps(x));
                x = x.offset(8);
                y = y.offset(8);
            });
        }
        for _ in 0..n % STEP {
            *y = *x;
            x = x.offset(1);
            y = y.offset(1);
        }
    } else {
        for _ in 0..n {
            *y = *x;
            x = x.offset(incx);
            y = y.offset(incy);
        }
    }
}

pub unsafe fn saxpy(
    n: usize,
    a: f32,
    mut x: *const f32,
    incx: isize,
    mut y: *mut f32,
    incy: isize,
) {
    if incx == 1 && incy == 1 {
        let av = _mm256_broadcast_ss(&a);
        for _ in 0..n / STEP {
            unroll4!({
                _mm256_storeu_ps(
                    y,
                    _mm256_fmadd_ps(av, _mm256_loadu_ps(x), _mm256_loadu_ps(y)),
                );
                x = x.offset(8);
                y = y.offset(8);
            });
        }
        for _ in 0..n % STEP {
            *y += a * *x;
            x = x.offset(1);
            y = y.offset(1);
        }
    } else {
        for _ in 0..n {
            *y += a * *x;
            x = x.offset(incx);
            y = y.offset(incy);
        }
    }
}

pub unsafe fn sdot(
    n: usize,
    mut x: *const f32,
    incx: isize,
    mut y: *const f32,
    incy: isize,
) -> f32 {
    if incx == 1 && incy == 1 {
        let mut acc = _mm256_setzero_ps();
        for _ in 0..n / STEP {
            unroll4!({
                acc = _mm256_fmadd_ps(_mm256_loadu_ps(x), _mm256_loadu_ps(y), acc);
                x = x.offset(8);
                y = y.offset(8);
            });
        }
        let mut acc = hadd_ps(acc);
        for _ in 0..n % STEP {
            acc += *x * *y;
            x = x.offset(1);
            y = y.offset(1);
        }
        acc
    } else {
        let mut acc = 0.0;
        for _ in 0..n {
            acc += *x * *y;
            x = x.offset(incx);
            y = y.offset(incy);
        }
        acc
    }
}

pub unsafe fn sdsdot(
    n: usize,
    b: f32,
    mut x: *const f32,
    incx: isize,
    mut y: *const f32,
    incy: isize,
) -> f32 {
    let mut acc: f64 = f64::from(b);
    for _ in 0..n {
        acc += f64::from(*x) * f64::from(*y);
        x = x.offset(incx);
        y = y.offset(incy);
    }
    acc as f32
}

pub unsafe fn snrm2(n: usize, mut x: *const f32, incx: isize) -> f32 {
    if incx == 1 {
        let mut acc = _mm256_setzero_ps();
        for _ in 0..n / STEP {
            unroll4!({
                let xv = _mm256_loadu_ps(x);
                acc = _mm256_fmadd_ps(xv, xv, acc);
                x = x.offset(8);
            });
        }
        let mut acc = hadd_ps(acc);
        for _ in 0..n % STEP {
            let xi = *x;
            acc += xi * xi;
            x = x.offset(1);
        }
        acc.sqrt()
    } else {
        let mut acc = 0.0;
        for _ in 0..n {
            let xi = *x;
            acc += xi * xi;
            x = x.offset(incx);
        }
        acc.sqrt()
    }
}

pub unsafe fn sasum(n: usize, mut x: *const f32, incx: isize) -> f32 {
    if incx == 1 {
        let mut acc = _mm256_setzero_ps();
        let mask = _mm256_broadcast_ss(&*(&SABS_MASK as *const u32 as *const f32));
        for _ in 0..n / STEP {
            unroll4!({
                acc = _mm256_add_ps(_mm256_and_ps(mask, _mm256_loadu_ps(x)), acc);
                x = x.offset(8);
            });
        }
        let mut acc = hadd_ps(acc);
        for _ in 0..n % STEP {
            acc += (*x).abs();
            x = x.offset(1);
        }
        acc
    } else {
        let mut acc = 0.0;
        for _ in 0..n {
            acc += (*x).abs();
            x = x.offset(incx);
        }
        acc
    }
}

pub unsafe fn isamax(n: usize, mut x: *const f32, incx: isize) -> usize {
    let mut max = 0.0;
    let mut imax = 0;
    for i in 0..n {
        let xi = (*x).abs();
        if xi > max {
            max = xi;
            imax = i;
        }
        x = x.offset(incx);
    }
    imax
}
